use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, SendError, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, thread};

use crate::extras::network_utils::unspecified_socket_to_loopback;
use crate::humpty_server::HumptyServer;
use crate::{error_log, info_log, warn_log, HumptyError};

/// Represents a handle to the simple TCP server app
pub struct App {
  main_thread: JoinHandle<()>,
  addr: SocketAddr,
  shutdown: SyncSender<()>,
  error_rx: Option<Receiver<AppError>>,
  done_rx: Option<Receiver<()>>,
}

impl App {
  /// Create a new App. Returns an io::Error if it was unable to bind to the socket.
  pub fn new(
    addr: impl ToSocketAddrs,
    humpty_server: Arc<HumptyServer>,
  ) -> Result<Self, io::Error> {
    let (shutdown_tx, shutdown_rx) = mpsc::sync_channel(1);
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let (error_tx, error_rx) = mpsc::sync_channel(1024);

    let tcp_listener = TcpListener::bind(addr)?;
    let addr = unspecified_socket_to_loopback(tcp_listener.local_addr()?)
      .expect("bound socket has a socket");
    info_log!("tcp_app: successfully listening on {}", addr);

    let main_thread = thread::spawn(move || {
      let mut threads = Vec::new();
      for stream in tcp_listener.incoming() {
        if shutdown_rx.try_recv().is_ok() {
          info_log!("tcp_app: shutdown receieved. breaking out of loop");
          break;
        }
        let humpty_server = humpty_server.clone();
        let error_tx = error_tx.clone();
        threads.push(thread::spawn(move || {
          let run = || {
            humpty_server.handle_connection(stream?)?;
            Ok::<(), AppError>(())
          };

          let res = run();
          if let Err(e) = res {
            error_log!("tcp_app: {:?} occurred", &e);
            if let Err(e) = error_tx.try_send(e) {
              warn_log!("tcp_app: unable to report error to the receiver, due to {}", e);
            }
          }
        }));

        // Iterate over the threads and remove the finished ones
        threads.retain(|handle| handle.is_finished());
      }

      // Wait for all threads to finish and/or timeout.
      for t in threads {
        let j = t.join();
        if let Err(e) = j {
          warn_log!("{:?} while doing join of `exec` thread.", e);
        }
      }

      if let Err(e) = done_tx.try_send(()) {
        warn_log!("tcp_app: unable to report done, due to {}", e);
      }
    });

    Ok(Self {
      addr,
      shutdown: shutdown_tx,
      error_rx: Some(error_rx),
      main_thread,
      done_rx: Some(done_rx),
    })
  }

  /// Request a shutdown. This will not exit until all connections have finished, which can be up to
  /// the duration of your `with_connection_timeout` for the HumptyServer.
  pub fn shutdown(self) -> Result<(), AppError> {
    info_log!("tcp_app: initiating shutdown.");
    self.shutdown.send(())?;
    info_log!("tcp_app: waking up main thread.");
    TcpStream::connect(self.addr)?;
    info_log!("tcp_app: waiting for main thread to join all threads.");
    let res = self.main_thread.join();
    if let Err(e) = res {
      error_log!("tcp_app: main thread panicked with {:?}.", &e);
      return Err(AppError::MainThreadFailure);
    }
    Ok(())
  }

  /// Receiver for errors, capped to 1024 errors.
  pub fn error_receiver(&mut self) -> Option<Receiver<AppError>> {
    self.error_rx.take()
  }

  /// Receiver to block until app is done from a `shutdown` call.
  pub fn done_receiver(&mut self) -> Option<Receiver<()>> {
    self.done_rx.take()
  }

  /// Runs, blocking forever as there is no way to call `shutdown`.
  /// If the `done_receiver` was already taken, this will return immediately
  pub fn run(self) {
    if let Some(done) = self.done_rx {
      let _ = done.recv();
    }
  }
}

#[derive(Debug)]
#[non_exhaustive]
/// Errors for this app. If you need more advanced control, this simple app is not suitable.
pub enum AppError {
  /// Main app thread has crashed/panicked, debug with `log` feature.
  MainThreadFailure,
  /// TCP network problem.
  IO(io::Error),
  /// Errors passed from HumptyServer's `handle_connection`.
  HumptyError(HumptyError),
}

impl From<SendError<()>> for AppError {
  fn from(_: SendError<()>) -> Self {
    AppError::MainThreadFailure
  }
}

impl From<io::Error> for AppError {
  fn from(err: io::Error) -> Self {
    AppError::IO(err)
  }
}

impl From<HumptyError> for AppError {
  fn from(err: HumptyError) -> Self {
    AppError::HumptyError(err)
  }
}
