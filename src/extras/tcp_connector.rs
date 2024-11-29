use crate::extras::network_utils::specify_socket_to_loopback;
use crate::humpty_error::HumptyResult;
use crate::humpty_server::HumptyServer;
use crate::{error_log, info_log, trace_log, warn_log};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

/// Represents a handle to the simple TCP server app
pub struct TcpConnector {
  inner: Arc<TcpConnectorInner>,
  shutdown_failed: AtomicBool,
  main_thread: JoinHandle<()>,
}

struct TcpConnectorInner {
  addr_string: String,
  addr: Vec<SocketAddr>,
  listener: TcpListener,
  shutdown_flag: AtomicBool,
  humpty_server: Arc<HumptyServer>,
}

struct ActiveConnection {
  id: u128,
  hdl: Option<JoinHandle<()>>,
}

impl TcpConnectorInner {
  fn run(&self) {
    let mut active_connection = Vec::<ActiveConnection>::with_capacity(1024);

    info_log!("tcp_connector[{}]: listening...", &self.addr_string);
    for (stream, this_connection) in self.listener.incoming().zip(1u128..) {
      if self.humpty_server.is_shutdown() || self.shutdown_flag.load(Ordering::SeqCst) {
        info_log!("tcp_connector[{}]: shutdown", &self.addr_string);
        break;
      }

      info_log!("tcp_connector[{}]: connection {this_connection} accepted", &self.addr_string);
      let path_clone = self.addr_string.clone();
      let server_clone = self.humpty_server.clone();

      match thread::Builder::new().spawn(move || {
        match stream {
          Ok(stream) => match server_clone.handle_connection(stream) {
            Ok(_) => {
              info_log!(
                "tcp_connector[{}]: connection {} processed successfully",
                path_clone,
                this_connection
              );
            }
            Err(err) => {
              // User code errored, like return Err in an Error handler.
              error_log!(
                "tcp_connector[{}]: connection {} humpty server returned err={}",
                path_clone,
                this_connection,
                err
              );
            }
          },
          Err(err) => {
            // This may just affect a single connection and is likely to recover on its own?
            error_log!(
              "tcp_connector[{}]: connection {} failed to accept a unix socket connection err={}",
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
          error_log!("tcp_connector[{}]: connection {} failed to spawn new thread to handle the connection err={}, will drop connection.", &self.addr_string, this_connection, err);
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
              "tcp_connector[{}]: connection {} thread panicked: {}",
              &self.addr_string,
              this_connection,
              msg
            );
          } else if let Some(msg) = err.downcast_ref::<String>() {
            error_log!(
              "tcp_connector[{}]: connection {} thread panicked: {}",
              &self.addr_string,
              this_connection,
              msg
            );
          } else {
            error_log!(
              "tcp_connector[{}]: connection {} thread panicked: {:?}",
              &self.addr_string,
              this_connection,
              err
            );
          };
        }

        false
      });
    }

    trace_log!("tcp_connector[{}]: waiting for shutdown to finish", &self.addr_string);
    //Wait for all threads to finish
    for mut con in active_connection {
      //Code for panic enjoyers
      if let Some(Err(err)) = con.hdl.take().map(JoinHandle::join) {
        let this_connection = con.id;
        if let Some(msg) = err.downcast_ref::<&'static str>() {
          error_log!(
            "tcp_connector[{}]: connection {} thread panicked: {}",
            &self.addr_string,
            this_connection,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "tcp_connector[{}]: connection {} thread panicked: {}",
            &self.addr_string,
            this_connection,
            msg
          );
        } else {
          error_log!(
            "tcp_connector[{}]: connection {} thread panicked: {:?}",
            &self.addr_string,
            this_connection,
            err
          );
        };
      }
    }

    info_log!("tcp_connector[{}]: shutdown done", &self.addr_string);
  }
}

impl TcpConnector {
  /// Create a new App. Returns an io::Error if it was unable to bind to the socket.
  pub fn new(addr: impl ToSocketAddrs, humpty_server: Arc<HumptyServer>) -> HumptyResult<Self> {
    let mut addr_string = String::new();
    let addr_in_vec = addr.to_socket_addrs()?.collect::<Vec<SocketAddr>>();

    for addr in &addr_in_vec {
      if !addr_string.is_empty() {
        addr_string += ", ";
      }
      addr_string += addr.to_string().as_str();
    }

    let inner = Arc::new(TcpConnectorInner {
      listener: TcpListener::bind(addr)?,
      shutdown_flag: AtomicBool::new(false),
      addr_string,
      humpty_server,
      addr: addr_in_vec,
    });

    let main_thread = {
      let inner = inner.clone();
      thread::Builder::new().spawn(move || {
        inner.run();
      })?
    };

    let connector =
      Self { inner: inner.clone(), shutdown_failed: AtomicBool::new(false), main_thread };
    Ok(connector)
  }

  /// Request a shutdown. This will not exit until all connections have finished, which can be up to
  /// the duration of your `with_connection_timeout` for the HumptyServer.
  ///
  #[allow(unsafe_code)]
  #[cfg(unix)]
  pub fn shutdown(&self) {
    use std::os::fd::AsRawFd;

    if self.inner.shutdown_flag.swap(true, Ordering::SeqCst) {
      return;
    }

    if self.main_thread.is_finished() {
      //No need to libc::shutdown() if somehow by magic the main_thread is already dead.
      return;
    }

    unsafe {
      if libc::shutdown(self.inner.listener.as_raw_fd(), libc::SHUT_RDWR) != -1 {
        return;
      }

      //This is very unlikely, I have NEVER seen this happen.
      let errno = *libc::__errno_location();
      error_log!("tcp_connector[{}]: shutdown failed: errno={}", &self.inner.addr_string, errno);
      self.shutdown_failed.store(true, Ordering::SeqCst);
    }
  }

  #[cfg(not(unix))]
  pub fn shutdown(&self) {
    self.shutdown_by_connecting();
  }

  /// This fn will set the shutdown flag and then attempt to open a tcp connection to
  /// the listening server (if it is still running) to wake it up.
  ///
  /// This is prone to failure because our lord and savior the Windows firewall may say no to this.
  /// On linux the netfilter kernel module might say no to this, if configured by a paranoid person
  /// (or someone is using RedHat linux).
  ///
  /// Should this be called on a system with a particularly nasty routing table configuration then
  /// the routing table may send the connection attempt into "nirvana". To ensure this doesn't happen
  /// the connection attempt is made with a timeout of 1s. Since any successfully made connection will be to localhost,
  /// this should be fine to handle this. If the routing table does indeed re-route this connection attempt to
  /// somewhere else that's not our server but is something that is running any kind of tcp server then there is no error
  /// to detect and this might cause join() to block until something successfully connects to our actual server.
  ///
  /// This is the best one can do given rusts standard library on windows.
  /// On Windows the shutdown fn will just call this fn, because there is nothing better that can be done.
  /// On Unix the shutdown fn is recommended as it will not establish a tcp connection to wake up the listener.
  ///
  /// Should wake up by establishing a connection fail then any call to join() will not be blocking
  /// and the background "thread" will keep waiting until eventually someone connects to it to wake it up.
  ///
  pub fn shutdown_by_connecting(&self) {
    if self.inner.shutdown_flag.swap(true, Ordering::SeqCst) {
      return;
    }

    if self.main_thread.is_finished() {
      return;
    }

    for addr in self.inner.addr.iter() {
      if self.main_thread.is_finished() {
        return;
      }
      let mut addr = *addr;
      specify_socket_to_loopback(&mut addr);
      if TcpStream::connect_timeout(&addr, Duration::from_millis(1000)).is_ok() {
        info_log!(
          "tcp_connector[{}]: connection to wakeup for shutdown was successful",
          &self.inner.addr_string
        );
        return;
      }
    }

    warn_log!("tcp_connector[{}]: connection to wakeup for shutdown was not successful, join() will not be blocking.", &self.inner.addr_string);
    self.shutdown_failed.store(true, Ordering::SeqCst);
  }

  /// Returns true if the unix connector is marked to shut down.
  /// join will possibly block forever if this fn returns false.
  pub fn is_shutdown(&self) -> bool {
    self.inner.shutdown_flag.load(Ordering::SeqCst)
  }

  /// Returns true if the unix connector is finished, join will not block if this fn returns true.
  pub fn is_finished(&self) -> bool {
    self.main_thread.is_finished()
  }

  /// Instructs the unix connector to shut down and blocks until all served connections are processed.
  pub fn shutdown_and_join(self) {
    self.shutdown();
    self.join()
  }

  /// Blocks, possibly forever, until the unix connector is done.
  pub fn join(self) {
    if self.shutdown_failed.load(Ordering::SeqCst) && !self.main_thread.is_finished() {
      warn_log!(
        "tcp_connector[{}]: due to previous failure of libc::shutdown join will not block",
        &self.inner.addr_string
      );
      return;
    }

    match self.main_thread.join() {
      Ok(_) => {}
      Err(err) => {
        if let Some(msg) = err.downcast_ref::<&'static str>() {
          error_log!(
            "tcp_connector[{}]: listener thread panicked: {}",
            &self.inner.addr_string,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "tcp_connector[{}]: listener thread panicked: {}",
            &self.inner.addr_string,
            msg
          );
        } else {
          error_log!(
            "tcp_connector[{}]: listener thread panicked: {:?}",
            &self.inner.addr_string,
            err
          );
        };
      }
    }
  }
}
