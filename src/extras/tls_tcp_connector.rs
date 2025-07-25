use crate::extras::connector::{ActiveConnection, ConnWait, ConnectorMeta};
use crate::extras::{
  Connector, CONNECTOR_SHUTDOWN_FLAG_POLLING_INTERVAL, CONNECTOR_SHUTDOWN_TIMEOUT,
};
use crate::functional_traits::{DefaultThreadAdapter, ThreadAdapter, ThreadAdapterJoinHandle};
use crate::tii_error::TiiResult;
use crate::tii_server::Server;
use crate::{error_log, info_log, trace_log, TlsStream};
use defer_heavy::defer;
use listener_poll::PollEx;
use rustls::{ServerConfig, ServerConnection};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Represents a handle to the simple TCP Socket Server that accepts connections and pumps them into Tii for handling.
#[derive(Debug)]
pub struct TlsTcpConnector {
  main_thread: Mutex<Option<ThreadAdapterJoinHandle>>,
  inner: Arc<TlsTcpConnectorInner>,
}

#[derive(Debug)]
struct TlsTcpConnectorInner {
  thread_adapter: Arc<dyn ThreadAdapter>,
  config: Arc<ServerConfig>,
  addr_string: String,
  waiter: ConnWait,
  listener: TcpListener,
  shutdown_flag: AtomicBool,
  tii_server: Arc<Server>,
}

impl TlsTcpConnectorInner {
  fn next(&self) -> io::Result<TcpStream> {
    loop {
      if self.tii_server.is_shutdown() || self.shutdown_flag.load(Ordering::SeqCst) {
        return Err(io::ErrorKind::ConnectionAborted.into());
      }

      if !self.listener.poll(Some(CONNECTOR_SHUTDOWN_FLAG_POLLING_INTERVAL))? {
        continue;
      }

      return self.listener.accept().map(|(stream, _)| stream);
    }
  }

  fn run(&self) {
    defer! {
      self.waiter.signal(2);
    }
    let mut active_connection = Vec::<ActiveConnection>::with_capacity(1024);

    info_log!("tii: tls_tcp_connector[{}]: listening...", &self.addr_string);
    for this_connection in 1u128.. {
      let stream = self.next();
      if self.tii_server.is_shutdown() || self.shutdown_flag.load(Ordering::SeqCst) {
        info_log!("tii: tls_tcp_connector[{}]: shutdown", &self.addr_string);
        break;
      }

      info_log!(
        "tii: tls_tcp_connector[{}]: connection {this_connection} accepted",
        &self.addr_string
      );
      let path_clone = self.addr_string.clone();
      let server_clone = self.tii_server.clone();
      let done_flag = Arc::new(AtomicBool::new(false));
      let done_clone = Arc::clone(&done_flag);
      let tls_config = self.config.clone();
      let thread_adapter_clone = self.thread_adapter.clone();

      match self.thread_adapter.spawn(Box::new(move || {
        defer! {
          done_clone.store(true, Ordering::SeqCst);
        }
        match stream {
          Ok(stream) => {
            let tls_stream = match ServerConnection::new(tls_config) {
              Ok(tls_con) => {
                match TlsStream::create(
                  stream,
                  tls_con,
                  thread_adapter_clone.as_ref(),
                ) {
                  Ok(conn) => conn,
                  Err(err) => {
                    error_log!(
                "tii: tls_tcp_connector[{}]: connection {} failed to construct TiiTlsStream err={}",
                path_clone,
                this_connection,
                err);
                    return;
                  }
                }
              }
              Err(err) => {
                error_log!(
                "tii: tls_tcp_connector[{}]: connection {} failed to construct rust-tls ServerConnection err={}",
                path_clone,
                this_connection,
                err);
                return;
              }
            };

            match server_clone.handle_connection_with_meta(tls_stream, ConnectorMeta::TlsTcp) {
              Ok(_) => {
                info_log!(
                "tii: tls_tcp_connector[{}]: connection {} processed successfully",
                path_clone,
                this_connection
              );
              }
              Err(err) => {
                // User code errored, like return Err in an Error handler.
                error_log!(
                "tii: tls_tcp_connector[{}]: connection {} tii server returned err={}",
                path_clone,
                this_connection,
                err
              );
              }
            }
          },
          Err(err) => {
            // This may just affect a single connection and is likely to recover on its own?
            error_log!(
              "tii: tls_tcp_connector[{}]: connection {} failed to accept a unix socket connection err={}",
              path_clone,
              this_connection,
              err
            );
          }
        }
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
          error_log!("tii: tls_tcp_connector[{}]: connection {} failed to spawn new thread to handle the connection err={}, will drop connection.", &self.addr_string, this_connection, err);
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
              "tii: tls_tcp_connector[{}]: connection {} thread panicked: {}",
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

    trace_log!("tii: tls_tcp_connector[{}]: waiting for shutdown to finish", &self.addr_string);
    //Wait for all threads to finish
    for mut con in active_connection {
      let this_connection = con.id;
      if !con.done_flag.load(Ordering::SeqCst) {
        trace_log!(
          "tii: tls_tcp_connector[{}]: connection {} is not yet done. blocking...",
          &self.addr_string,
          this_connection
        );
      }

      //Code for panic enjoyers
      if let Some(Err(err)) = con.hdl.take().map(ThreadAdapterJoinHandle::join) {
        crate::util::panic_msg(err, |msg| {
          error_log!(
            "tii: tls_tcp_connector[{}]: connection {} thread panicked: {}",
            &self.addr_string,
            this_connection,
            msg
          );
        });
      }
    }

    info_log!("tii: tls_tcp_connector[{}]: shutdown done", &self.addr_string);
  }
}

impl TlsTcpConnectorInner {
  pub fn shutdown(&self) {
    if self.shutdown_flag.swap(true, Ordering::SeqCst) {
      return;
    }

    if !self.waiter.wait(1, Some(CONNECTOR_SHUTDOWN_TIMEOUT)) {
      error_log!(
        "tls_tcp_connector[{}]: shutdown failed to wake up the listener thread",
        &self.addr_string
      );
    }
  }
}
impl Connector for TlsTcpConnector {
  fn shutdown(&self) {
    self.inner.shutdown();
  }

  fn is_marked_for_shutdown(&self) -> bool {
    self.inner.shutdown_flag.load(Ordering::SeqCst)
  }

  fn is_shutting_down(&self) -> bool {
    self.inner.waiter.is_done(2)
  }

  fn is_shutdown(&self) -> bool {
    self.inner.waiter.is_done(2)
  }

  fn shutdown_and_join(&self, timeout: Option<Duration>) -> bool {
    self.shutdown();
    self.join(timeout)
  }

  fn join(&self, timeout: Option<Duration>) -> bool {
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
            "tii: tls_tcp_connector[{}]: listener thread panicked: {}",
            &self.inner.addr_string,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "tii: tls_tcp_connector[{}]: listener thread panicked: {}",
            &self.inner.addr_string,
            msg
          );
        } else {
          error_log!(
            "tii: tls_tcp_connector[{}]: listener thread panicked: {:?}",
            &self.inner.addr_string,
            err
          );
        };
      }
    }

    true
  }
}

impl TlsTcpConnector {
  /// Creates a new tls connector that is listening on the given addr.
  /// Return Err on error.
  /// The TCP listener will listen immediately in a background thread.
  pub fn start(
    addr: impl ToSocketAddrs,
    tii_server: Arc<Server>,
    config: Arc<ServerConfig>,
    thread_adapter: impl ThreadAdapter + 'static,
  ) -> TiiResult<Self> {
    //Check if the rust-tls server config is "valid".
    let _ = ServerConnection::new(config.clone())?;

    let mut addr_string = String::new();
    let addr_in_vec = addr.to_socket_addrs()?.collect::<Vec<SocketAddr>>();

    for addr in &addr_in_vec {
      if !addr_string.is_empty() {
        addr_string += ", ";
      }
      addr_string += addr.to_string().as_str();
    }

    let thread_adapter = Arc::new(thread_adapter);
    let inner = Arc::new(TlsTcpConnectorInner {
      thread_adapter: thread_adapter.clone(),
      config,
      listener: TcpListener::bind(addr)?,
      shutdown_flag: AtomicBool::new(false),
      addr_string,
      tii_server: tii_server.clone(),
      waiter: ConnWait::default(),
    });

    inner.listener.set_nonblocking(true)?;

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

  /// Create a new TlsConnector.
  /// When this fn returns Ok() the socket is already listening in a background thread.
  /// Returns an io::Error if it was unable to bind to the socket.
  ///
  /// Threads are created using "thread::Builder::new().spawn"
  pub fn start_unpooled(
    addr: impl ToSocketAddrs,
    config: Arc<ServerConfig>,
    tii_server: Arc<Server>,
  ) -> TiiResult<Self> {
    Self::start(addr, tii_server, config, DefaultThreadAdapter)
  }
}
