use crate::extras::connector::{ActiveConnection, ConnWait};
use crate::extras::{Connector, CONNECTOR_SHUTDOWN_TIMEOUT};
use crate::humpty_error::HumptyResult;
use crate::humpty_server::HumptyServer;
use crate::{error_log, info_log, trace_log, HumptyError};
use defer_heavy::defer;
use socket2::{Domain, Socket, Type};
use std::io::ErrorKind;
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

/// Represents a handle to the simple TCP server app
#[derive(Debug)]
pub struct Socket2TcpConnector {
  main_thread: Mutex<Option<JoinHandle<()>>>,
  inner: Arc<Socket2TcpConnectorInner>,
}

#[derive(Debug)]
struct Socket2TcpConnectorInner {
  addr_string: String,
  waiter: ConnWait,
  listener: Socket,
  shutdown_flag: AtomicBool,
  humpty_server: Arc<HumptyServer>,
}

impl Socket2TcpConnectorInner {
  fn run(&self) {
    defer! {
      self.waiter.signal(2);
    }
    let mut active_connection = Vec::<ActiveConnection>::with_capacity(1024);

    info_log!("socket2_tcp_connector[{}]: listening...", &self.addr_string);
    for this_connection in 1u128.. {
      let accepted = self.listener.accept();
      if self.humpty_server.is_shutdown() || self.shutdown_flag.load(Ordering::SeqCst) {
        info_log!("socket2_tcp_connector[{}]: shutdown", &self.addr_string);
        break;
      }

      info_log!(
        "socket2_tcp_connector[{}]: connection {this_connection} accepted",
        &self.addr_string
      );
      let path_clone = self.addr_string.clone();
      let server_clone = self.humpty_server.clone();

      match thread::Builder::new().spawn(move || {
        match accepted {
          Ok((stream, _)) => {
            let tcp_stream : TcpStream = stream.into();
            match server_clone.handle_connection(tcp_stream) {
              Ok(_) => {
                info_log!(
                "socket2_tcp_connector[{}]: connection {} processed successfully",
                path_clone,
                this_connection
              );
              }
              Err(err) => {
                // User code errored, like return Err in an Error handler.
                error_log!(
                "socket2_tcp_connector[{}]: connection {} humpty server returned err={}",
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
              "socket2_tcp_connector[{}]: connection {} failed to accept a unix socket connection err={}",
              path_clone,
              this_connection,
              err
            );
          }
        }
      }) {
        Ok(handle) => {
          active_connection.push(ActiveConnection { id: this_connection, hdl: Some(handle) });
        }
        Err(err) => {
          //May recover on its own courtesy of the OS once load decreases.
          error_log!("socket2_tcp_connector[{}]: connection {} failed to spawn new thread to handle the connection err={}, will drop connection.", &self.addr_string, this_connection, err);
        }
      }

      active_connection.retain_mut(|con| {
        match con.hdl.as_ref() {
          Some(hdl) => {
            if !hdl.is_finished() {
              return true;
            }
          }
          None => return false,
        }

        //Code for panic enjoyers
        if let Some(Err(err)) = con.hdl.take().map(JoinHandle::join) {
          let this_connection = con.id;
          if let Some(msg) = err.downcast_ref::<&'static str>() {
            error_log!(
              "socket2_tcp_connector[{}]: connection {} thread panicked: {}",
              &self.addr_string,
              this_connection,
              msg
            );
          } else if let Some(msg) = err.downcast_ref::<String>() {
            error_log!(
              "socket2_tcp_connector[{}]: connection {} thread panicked: {}",
              &self.addr_string,
              this_connection,
              msg
            );
          } else {
            error_log!(
              "socket2_tcp_connector[{}]: connection {} thread panicked: {:?}",
              &self.addr_string,
              this_connection,
              err
            );
          };
        }

        false
      });
    }

    self.waiter.signal(1);

    trace_log!("socket2_tcp_connector[{}]: waiting for shutdown to finish", &self.addr_string);
    //Wait for all threads to finish
    for mut con in active_connection {
      //Code for panic enjoyers
      if let Some(Err(err)) = con.hdl.take().map(JoinHandle::join) {
        let this_connection = con.id;
        if let Some(msg) = err.downcast_ref::<&'static str>() {
          error_log!(
            "socket2_tcp_connector[{}]: connection {} thread panicked: {}",
            &self.addr_string,
            this_connection,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "socket2_tcp_connector[{}]: connection {} thread panicked: {}",
            &self.addr_string,
            this_connection,
            msg
          );
        } else {
          error_log!(
            "socket2_tcp_connector[{}]: connection {} thread panicked: {:?}",
            &self.addr_string,
            this_connection,
            err
          );
        };
      }
    }

    info_log!("socket2_tcp_connector[{}]: shutdown done", &self.addr_string);
  }
}

impl Connector for Socket2TcpConnector {
  fn shutdown(&self) {
    if self.inner.shutdown_flag.swap(true, Ordering::SeqCst) {
      return;
    }

    if self.inner.waiter.is_done(1) {
      //No need to libc::shutdown() if somehow by magic the main_thread is already dead.
      return;
    }

    if let Err(err) = self.inner.listener.shutdown(Shutdown::Both) {
      if self.inner.waiter.wait(1, Some(CONNECTOR_SHUTDOWN_TIMEOUT)) {
        return;
      }

      error_log!(
        "socket2_tcp_connector[{}]: failed to shutdown listener: {}",
        &self.inner.addr_string,
        err
      );
      return;
    }

    if self.inner.waiter.wait(1, Some(CONNECTOR_SHUTDOWN_TIMEOUT)) {
      return;
    }

    error_log!(
      "socket2_tcp_connector[{}]: failed to wakeup listener thread",
      &self.inner.addr_string
    );
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
            "socket2_tcp_connector[{}]: listener thread panicked: {}",
            &self.inner.addr_string,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "socket2_tcp_connector[{}]: listener thread panicked: {}",
            &self.inner.addr_string,
            msg
          );
        } else {
          error_log!(
            "socket2_tcp_connector[{}]: listener thread panicked: {:?}",
            &self.inner.addr_string,
            err
          );
        };
      }
    }

    true
  }
}

impl Socket2TcpConnector {
  /// Create a new Connector. Returns an io::Error if it was unable to bind to the socket.
  pub fn start(addr: impl ToSocketAddrs, humpty_server: Arc<HumptyServer>) -> HumptyResult<Self> {
    let mut addr_in_vec = addr.to_socket_addrs()?.collect::<Vec<SocketAddr>>();

    if addr_in_vec.len() > 1 {
      return Err(HumptyError::from_io_kind(ErrorKind::ArgumentListTooLong));
    }

    let Some(addr) = addr_in_vec.pop() else {
      return Err(HumptyError::from_io_kind(ErrorKind::AddrNotAvailable));
    };

    let addr_string = addr.to_string();

    let addr = socket2::SockAddr::from(addr);

    let socket = if addr.is_ipv6() {
      Socket::new(Domain::IPV6, Type::STREAM, None)?
    } else if addr.is_ipv4() {
      Socket::new(Domain::IPV4, Type::STREAM, None)?
    } else {
      return Err(HumptyError::from_io_kind(ErrorKind::AddrNotAvailable));
    };

    socket.set_reuse_address(true)?;
    socket.bind(&addr)?;
    socket.listen(128)?;

    let inner = Arc::new(Socket2TcpConnectorInner {
      listener: socket,
      shutdown_flag: AtomicBool::new(false),
      addr_string,
      humpty_server,
      waiter: ConnWait::default(),
    });

    let main_thread = {
      let inner = inner.clone();
      thread::Builder::new().spawn(move || {
        inner.run();
      })?
    };

    let connector = Self { main_thread: Mutex::new(Some(main_thread)), inner: inner.clone() };
    Ok(connector)
  }
}
