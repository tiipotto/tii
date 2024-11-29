use crate::humpty_error::HumptyResult;
use crate::humpty_server::HumptyServer;
use crate::{error_log, info_log, trace_log, warn_log};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

/// Represents a handle to the simple Unix Socket Server that accepts connections and pumps them into Humpty for handling.
pub struct UnixConnector {
  inner: Arc<UnixConnectorInner>,
  shutdown_failed: AtomicBool,
  main_thread: JoinHandle<()>,
}

struct UnixConnectorInner {
  path: PathBuf,
  listener: UnixListener,
  shutdown_flag: AtomicBool,
  humpty_server: Arc<HumptyServer>,
}

struct ActiveConnection {
  id: u128,
  hdl: Option<JoinHandle<()>>,
}

impl UnixConnectorInner {
  fn run(&self) {
    let mut active_connection = Vec::<ActiveConnection>::with_capacity(1024);

    info_log!("unix_connector[{}]: listening...", self.path.display());
    for (stream, this_connection) in self.listener.incoming().zip(1u128..) {
      if self.humpty_server.is_shutdown() || self.shutdown_flag.load(Ordering::SeqCst) {
        info_log!("unix_connector[{}]: shutdown", self.path.display());
        break;
      }

      info_log!("unix_connector[{}]: connection {this_connection} accepted", self.path.display());
      let path_clone = self.path.clone();
      let server_clone = self.humpty_server.clone();

      match thread::Builder::new().spawn(move || {
        match stream {
          Ok(stream) => match server_clone.handle_connection(stream) {
            Ok(_) => {
              info_log!(
                "unix_connector[{}]: connection {this_connection} processed successfully",
                path_clone.display()
              );
            }
            Err(err) => {
              // User code errored, like return Err in an Error handler.
              error_log!(
                "unix_connector[{}]: connection {} humpty server returned err={}",
                path_clone.display(),
                this_connection,
                err
              );
            }
          },
          Err(err) => {
            // This may just affect a single connection and is likely to recover on its own?
            error_log!(
              "unix_connector[{}]: connection {} failed to accept a unix socket connection err={}",
              path_clone.display(),
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
          error_log!("unix_connector[{}]: connection {} failed to spawn new thread to handle the connection err={}, will drop connection.", self.path.display(), err, this_connection);
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
              "unix_connector[{}]: connection {} thread panicked: {}",
              self.path.display(),
              this_connection,
              msg
            );
          } else if let Some(msg) = err.downcast_ref::<String>() {
            error_log!(
              "unix_connector[{}]: connection {} thread panicked: {}",
              self.path.display(),
              this_connection,
              msg
            );
          } else {
            error_log!(
              "unix_connector[{}]: connection {} thread panicked: {:?}",
              self.path.display(),
              this_connection,
              err
            );
          };
        }

        false
      });
    }

    trace_log!("unix_connector[{}]: waiting for shutdown to finish", self.path.display());
    //Wait for all threads to finish
    for mut con in active_connection {
      //Code for panic enjoyers
      if let Some(Err(err)) = con.hdl.take().map(JoinHandle::join) {
        let this_connection = con.id;
        if let Some(msg) = err.downcast_ref::<&'static str>() {
          error_log!(
            "unix_connector[{}]: connection {} thread panicked: {}",
            self.path.display(),
            this_connection,
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "unix_connector[{}]: connection {} thread panicked: {}",
            self.path.display(),
            this_connection,
            msg
          );
        } else {
          error_log!(
            "unix_connector[{}]: connection {this_connection} thread panicked: {:?}",
            self.path.display(),
            err
          );
        };
      }
    }

    info_log!("unix_connector[{}]: shutdown done", self.path.display());
  }
}

impl UnixConnector {
  /// Create a new App. Returns an io::Error if it was unable to bind to the socket.
  pub fn new(addr: impl AsRef<Path>, humpty_server: Arc<HumptyServer>) -> HumptyResult<Self> {
    let path = addr.as_ref();
    if std::fs::exists(path)? {
      std::fs::remove_file(path)?;
    }

    let inner = Arc::new(UnixConnectorInner {
      listener: UnixListener::bind(path)?,
      shutdown_flag: AtomicBool::new(false),
      path: path.to_path_buf(),
      humpty_server,
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
  pub fn shutdown(&self) {
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
      error_log!("unix_connector[{}]: shutdown failed: errno={}", self.inner.path.display(), errno);
      self.shutdown_failed.store(true, Ordering::SeqCst);
    }
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
        "unix_connector[{}]: due to previous failure of libc::shutdown join will not block",
        self.inner.path.display()
      );
      return;
    }

    match self.main_thread.join() {
      Ok(_) => {}
      Err(err) => {
        if let Some(msg) = err.downcast_ref::<&'static str>() {
          error_log!(
            "unix_connector[{}]: listener thread panicked: {}",
            self.inner.path.display(),
            msg
          );
        } else if let Some(msg) = err.downcast_ref::<String>() {
          error_log!(
            "unix_connector[{}]: listener thread panicked: {}",
            self.inner.path.display(),
            msg
          );
        } else {
          error_log!(
            "unix_connector[{}]: listener thread panicked: {:?}",
            self.inner.path.display(),
            err
          );
        };
      }
    }
  }
}
