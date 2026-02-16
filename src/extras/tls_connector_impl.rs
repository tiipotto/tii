pub(crate) trait Adapter<Listener: Send + Sync + 'static, Stream: Send + Sync + 'static>:
  Send + Sync + 'static
{
  fn listener_set_nonblocking(listener: &Listener, flag: bool) -> io::Result<()>;
  fn stream_set_nonblocking(stream: &Stream, flag: bool) -> io::Result<()>;
  fn set_read_timeout(stream: &Stream, timeout: Option<Duration>) -> io::Result<()>;
  fn read_exact(stream: &Stream, buf: &mut [u8]) -> io::Result<()>;
  fn accept(listener: &Listener, tii: &Server, shutdown_flag: &AtomicBool) -> io::Result<Stream>;
  fn tls(
    stream: Stream,
    initial_data: &[u8],
    scon: ServerConnection,
    spawner: &dyn ThreadAdapter,
  ) -> io::Result<Box<dyn ConnectionStream>>;
  fn plain(stream: Stream, initial_data: &[u8]) -> Box<dyn ConnectionStream>;
  fn meta_tls() -> ConnectorMeta;
  fn meta_plain() -> ConnectorMeta;
}

use crate::extras::connector::{ActiveConnection, ConnWait, ConnectorMeta};
use crate::extras::CONNECTOR_SHUTDOWN_TIMEOUT;
use crate::functional_traits::{ThreadAdapter, ThreadAdapterJoinHandle};
use crate::tii_error::TiiResult;
use crate::tii_server::Server;
use crate::{error_log, info_log, trace_log, ConnectionStream};
use defer_heavy::defer;
use rustls::{ServerConfig, ServerConnection};
use std::io;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Represents a handle to the simple TCP Socket Server that accepts connections and pumps them into Tii for handling.
#[derive(Debug)]
pub struct TlsConnectorImpl<
  Listener: Send + Sync + 'static,
  Stream: Send + 'static + Sync,
  A: Adapter<Listener, Stream>,
> {
  main_thread: Mutex<Option<ThreadAdapterJoinHandle>>,
  inner: Arc<TlsConnectorInner<Listener, Stream, A>>,
}

#[derive(Debug)]
struct TlsConnectorInner<
  Listener: Send + Sync + 'static,
  Stream: Send + 'static + Sync,
  A: Adapter<Listener, Stream>,
> {
  log_name: &'static str,
  thread_adapter: Arc<dyn ThreadAdapter>,
  config: Arc<ServerConfig>,
  addr_string: String,
  waiter: ConnWait,
  listener: Listener,
  shutdown_flag: AtomicBool,
  tii_server: Arc<Server>,
  permit_plain_text: bool,
  _phantom1: PhantomData<Stream>,
  _phantom2: PhantomData<A>,
}

#[derive(Debug)]
struct TlsConnectorConnectionHandler<
  Listener: Send + Sync + 'static,
  Stream: Send + 'static + Sync,
  A: Adapter<Listener, Stream>,
> {
  log_name: &'static str,
  done_clone: Arc<AtomicBool>,
  path_clone: String,
  this_connection: u128,
  server_clone: Arc<Server>,
  tls_config: Arc<ServerConfig>,
  thread_adapter: Arc<dyn ThreadAdapter>,
  permit_plain_text: bool,
  _phantom1: PhantomData<Listener>,
  _phantom2: PhantomData<A>,
  _phantom3: PhantomData<Stream>,
}
impl<Listener: Send + Sync, Stream: Send + 'static + Sync, A: Adapter<Listener, Stream>>
  TlsConnectorConnectionHandler<Listener, Stream, A>
{
  fn handle_tls_connection(&self, stream: Stream, initial_data: &[u8]) {
    let tls_stream = match ServerConnection::new(self.tls_config.clone()) {
      Ok(tls_con) => match A::tls(stream, initial_data, tls_con, self.thread_adapter.as_ref()) {
        Ok(conn) => conn,
        Err(err) => {
          error_log!(
            "tii: {}[{}]: connection {} failed to construct TiiTlsStream err={}",
            self.log_name,
            &self.path_clone,
            self.this_connection,
            err
          );
          return;
        }
      },
      Err(err) => {
        error_log!(
          "tii: {}[{}]: connection {} failed to construct rust-tls ServerConnection err={}",
          self.log_name,
          &self.path_clone,
          self.this_connection,
          err
        );
        return;
      }
    };

    match self.server_clone.handle_connection_with_meta(tls_stream, A::meta_tls()) {
      Ok(_) => {
        info_log!(
          "tii: {}[{}]: connection {} processed successfully",
          self.log_name,
          &self.path_clone,
          self.this_connection
        );
      }
      Err(err) => {
        // User code errored, like return Err in an Error handler.
        error_log!(
          "tii: {}[{}]: connection {} tii server returned err={}",
          self.log_name,
          &self.path_clone,
          self.this_connection,
          err
        );
      }
    }
  }

  fn handle_plain_text_connection(&self, stream: Stream, first_byte: u8) {
    let stream = A::plain(stream, &[first_byte]);
    match self.server_clone.handle_connection_with_meta(stream, A::meta_plain()) {
      Ok(_) => {
        info_log!(
          "tii: {}[{}]: plain text connection {} processed successfully",
          self.log_name,
          &self.path_clone,
          self.this_connection
        );
      }
      Err(err) => {
        // User code errored, like return Err in an Error handler.
        error_log!(
          "tii: {}[{}]: plain text connection {} tii server returned err={}",
          self.log_name,
          &self.path_clone,
          self.this_connection,
          err
        );
      }
    }
  }
  fn handle_stream_in_thread(&self, stream: io::Result<Stream>) {
    defer! {
      self.done_clone.store(true, Ordering::SeqCst);
    }
    match stream {
      Ok(stream) => {
        // Why are we even using the standard library at this point whe it's non-portable.
        // This call is not needed on linux but is needed on windows.
        // See https://github.com/rust-lang/rust/issues/67027
        if let Err(err) = A::stream_set_nonblocking(&stream, false) {
          error_log!(
            "tii: {}[{}]: connection {} failed to call TcpStream::set_nonblocking(false) err={}",
            self.log_name,
            &self.path_clone,
            self.this_connection,
            err
          );
          return;
        }

        if !self.permit_plain_text {
          self.handle_tls_connection(stream, &[]);
          return;
        }

        if let Some(timeout) = self.server_clone.connection_timeout() {
          //TODO do I even need this if or should i just call set_timeout(None)????
          if let Err(err) = A::set_read_timeout(&stream, Some(timeout)) {
            error_log!(
                  "tii: {}[{}]: connection {} failed to call TcpStream::set_read_timeout(Some({:?})) err={}",
              self.log_name,
                  &self.path_clone,
                  self.this_connection,
                  timeout,
                  err
                );
          }
        }

        let mut inital_data = [0u8];
        if let Err(err) = A::read_exact(&stream, &mut inital_data) {
          error_log!(
            "tii: {}[{}]: connection {} failed to read first byte from connection err={}",
            self.log_name,
            &self.path_clone,
            self.this_connection,
            err
          );
        }
        let first_byte = inital_data[0];

        //https://tls12.xargs.org/#client-hello
        //0x16 just so happens to not be a printable ascii char, so it can't be the first ASCII character of an http method.
        if first_byte != 0x16 {
          info_log!(
              "tii: {}[{}]: connection {} client is requesting a plain text connection. Will not do tls for this connection.",
          self.log_name,
              &self.path_clone,
              self.this_connection,
            );
          self.handle_plain_text_connection(stream, first_byte);
          return;
        }

        self.handle_tls_connection(stream, &[0x16]);
      }
      Err(err) => {
        // This may just affect a single connection and is likely to recover on its own?
        error_log!(
          "tii: {}[{}]: connection {} failed to accept a unix socket connection err={}",
          self.log_name,
          &self.path_clone,
          self.this_connection,
          err
        );
      }
    }
  }
}

impl<Listener: Send + Sync, Stream: Send + 'static + Sync, A: Adapter<Listener, Stream>>
  TlsConnectorInner<Listener, Stream, A>
{
  fn run(&self) {
    defer! {
      self.waiter.signal(2);
    }
    let mut active_connection = Vec::<ActiveConnection>::with_capacity(1024);

    info_log!("tii: {}[{}]: listening...", self.log_name, &self.addr_string);
    for this_connection in 1u128.. {
      let stream = A::accept(&self.listener, &self.tii_server, &self.shutdown_flag);
      if self.tii_server.is_shutdown() || self.shutdown_flag.load(Ordering::SeqCst) {
        info_log!("tii: {}[{}]: shutdown", self.log_name, &self.addr_string);
        break;
      }

      info_log!(
        "tii: {}[{}]: connection {this_connection} accepted",
        self.log_name,
        &self.addr_string
      );

      let done_flag = Arc::new(AtomicBool::new(false));

      let thread_data = TlsConnectorConnectionHandler::<Listener, Stream, A> {
        log_name: self.log_name,
        done_clone: done_flag.clone(),
        path_clone: self.addr_string.clone(),
        this_connection,
        server_clone: self.tii_server.clone(),
        tls_config: self.config.clone(),
        thread_adapter: self.thread_adapter.clone(),
        permit_plain_text: self.permit_plain_text,
        _phantom1: Default::default(),
        _phantom2: Default::default(),
        _phantom3: Default::default(),
      };

      match self.thread_adapter.spawn(Box::new(move || {
        thread_data.handle_stream_in_thread(stream);
      })) {
        Ok(handle) => {
          active_connection.push(ActiveConnection {
            id: this_connection,
            hdl: Some(handle),
            done_flag,
          });
        }
        Err(err) => {
          //May recover on its own courtesy of the OS once load decreases.
          error_log!("tii: {}[{}]: connection {} failed to spawn new thread to handle the connection err={}, will drop connection.", self.log_name, &self.addr_string, this_connection, err);
        }
      }

      active_connection.retain_mut(|con| {
        if !con.done_flag.load(Ordering::SeqCst) {
          return true;
        }
        if con.hdl.is_none() {
          return false;
        }

        //Code for panic enjoyers
        if let Some(Err(err)) = con.hdl.take().map(ThreadAdapterJoinHandle::join) {
          let this_connection = con.id;
          crate::util::panic_msg(err, |msg| {
            error_log!(
              "tii: {}[{}]: connection {} thread panicked: {}",
              self.log_name,
              &self.addr_string,
              this_connection,
              msg
            );
          });
        }

        false
      });
    }

    self.waiter.signal(1);

    trace_log!("tii: {}[{}]: waiting for shutdown to finish", self.log_name, &self.addr_string);
    //Wait for all threads to finish
    for mut con in active_connection {
      let this_connection = con.id;
      if !con.done_flag.load(Ordering::SeqCst) {
        trace_log!(
          "tii: {}[{}]: connection {} is not yet done. blocking...",
          self.log_name,
          &self.addr_string,
          this_connection
        );
      }

      //Code for panic enjoyers
      if let Some(Err(err)) = con.hdl.take().map(ThreadAdapterJoinHandle::join) {
        crate::util::panic_msg(err, |msg| {
          error_log!(
            "tii: {}[{}]: connection {} thread panicked: {}",
            self.log_name,
            &self.addr_string,
            this_connection,
            msg
          );
        });
      }
    }

    info_log!("tii: {}[{}]: shutdown done", self.log_name, &self.addr_string);
  }

  pub fn shutdown(&self) {
    if self.shutdown_flag.swap(true, Ordering::SeqCst) {
      return;
    }

    if !self.waiter.wait(1, Some(CONNECTOR_SHUTDOWN_TIMEOUT)) {
      error_log!(
        "{}[{}]: shutdown failed to wake up the listener thread",
        self.log_name,
        &self.addr_string
      );
    }
  }
}

impl<Listener: Send + Sync, Stream: Send + 'static + Sync, A: Adapter<Listener, Stream>>
  TlsConnectorImpl<Listener, Stream, A>
{
  pub fn shutdown(&self) {
    self.inner.shutdown();
  }

  pub fn is_marked_for_shutdown(&self) -> bool {
    self.inner.shutdown_flag.load(Ordering::SeqCst)
  }

  pub fn is_shutting_down(&self) -> bool {
    self.inner.waiter.is_done(2)
  }

  pub fn is_shutdown(&self) -> bool {
    self.inner.waiter.is_done(2)
  }

  pub fn shutdown_and_join(&self, timeout: Option<Duration>) -> bool {
    self.shutdown();
    self.join(timeout)
  }

  pub fn join(&self, timeout: Option<Duration>) -> bool {
    if !self.inner.waiter.wait(2, timeout) {
      return false;
    }

    let Ok(mut guard) = self.main_thread.lock() else {
      return false;
    };

    let Some(join_handle) = guard.take() else {
      return true;
    };

    match join_handle.join() {
      Ok(_) => (),
      Err(err) => {
        if let Some(msg) = err.downcast_ref::<&'static str>() {
          error_log!(
            "tii: {}[{}]: listener thread panicked: {}",
            self.inner.log_name,
            &self.inner.addr_string,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "tii: {}[{}]: listener thread panicked: {}",
            self.inner.log_name,
            &self.inner.addr_string,
            msg
          );
        } else {
          error_log!(
            "tii: {}[{}]: listener thread panicked: {:?}",
            self.inner.log_name,
            &self.inner.addr_string,
            err
          );
        };
      }
    }

    true
  }

  /// Creates a new tls connector that is listening on the given addr.
  /// Return Err on error.
  /// The TCP listener will listen immediately in a background thread.
  ///
  pub fn start(
    log_name: &'static str,
    addr: String,
    listener: Listener,
    tii_server: Arc<Server>,
    config: Arc<ServerConfig>,
    thread_adapter: impl ThreadAdapter + 'static,
    permit_plain_text: bool,
  ) -> TiiResult<Self> {
    //Check if the rust-tls server config is "valid".
    let _ = ServerConnection::new(config.clone())?;

    let thread_adapter = Arc::new(thread_adapter);
    let inner = Arc::new(TlsConnectorInner {
      log_name,
      thread_adapter: thread_adapter.clone(),
      config,
      listener,
      shutdown_flag: AtomicBool::new(false),
      addr_string: addr,
      tii_server: tii_server.clone(),
      waiter: ConnWait::default(),
      permit_plain_text,
      _phantom1: Default::default(),
      _phantom2: Default::default(),
    });

    A::listener_set_nonblocking(&inner.listener, true)?;

    let main_thread = {
      let inner = inner.clone();
      thread_adapter.spawn(Box::new(move || {
        inner.run();
      }))?
    };

    let connector = Self { main_thread: Mutex::new(Some(main_thread)), inner: inner.clone() };

    let weak_inner = Arc::downgrade(&inner);

    tii_server.add_shutdown_hook(move || {
      if let Some(inner) = weak_inner.upgrade() {
        inner.shutdown()
      }
    });

    Ok(connector)
  }
}
