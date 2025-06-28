use crate::tii_builder::ThreadAdapterJoinHandle;
use crate::tii_server::ConnectionStreamMetadata;
use std::any::Any;
use std::fmt::{Display, Formatter};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// This constant contains the amount of time to wait to confirm that a connector did begin shutting down.
/// Considerations for this value are the time it takes to connect to localhost, the time for the scheduler to wake up
/// the listener thread and the time it takes for the listener thread to process a few of lines of code.
///
/// If this value is too small:
/// Worst case is that we fail to wake up the listener thread.
/// Otherwise, is that we log an error and later succeed.
///
/// If this value is too big:
/// We may block for this amount of time without the user of tii expecting it.
pub(crate) const CONNECTOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// See above
pub(crate) const CONNECTOR_SHUTDOWN_FLAG_POLLING_INTERVAL: Duration = Duration::from_secs(1);

/// Trait that defines all fn's that each connector implemented by tii::extras has.
pub trait Connector {
  /// Request a shutdown.
  /// This will not interrupt/stop open connections.
  fn shutdown(&self);

  /// Returns true if the unix connector is marked to shut down.
  /// join will possibly block forever if this fn returns false.
  fn is_marked_for_shutdown(&self) -> bool;

  /// Returns true if the connector is currently waiting for open connections to finish.
  fn is_shutting_down(&self) -> bool;

  /// Returns true if the unix connector is fully shutdown, join will not block if this fn returns true.
  fn is_shutdown(&self) -> bool;

  /// Instructs the unix connector to shut down and blocks until all served connections are processed.
  /// returns true if the shutdown is completed, false if timeout occurred.
  /// If this fn returned false the shutdown will continue in the background and join can be called again to await it.
  fn shutdown_and_join(&self, timeout: Option<Duration>) -> bool;

  /// Blocks, possibly forever, until the connector is done.
  /// If this fn returned true then the shutdown is completed, false if timeout occurred.
  /// This fn does not stop an ongoing shutdown if it times out.
  fn join(&self, timeout: Option<Duration>) -> bool;
}

///Metadata type appended by the extras Tii Connectors.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConnectorMeta {
  /// a TcpConnector made this connection
  Tcp,

  /// a TlsTcpConnector made this connection
  #[cfg(feature = "tls")]
  TlsTcp,

  /// a UnixConnector made this connection
  #[cfg(unix)]
  Unix,
  /// a TlsUnixConnector made this connection
  #[cfg(unix)]
  #[cfg(feature = "tls")]
  TlsUnix,
}

impl ConnectionStreamMetadata for ConnectorMeta {
  fn as_any(&self) -> &dyn Any {
    self
  }
}
impl Display for ConnectorMeta {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO improve
    std::fmt::Debug::fmt(self, f)
  }
}

#[derive(Debug)]
pub(crate) struct ActiveConnection {
  pub(crate) id: u128,
  pub(crate) hdl: Option<ThreadAdapterJoinHandle>,
  pub(crate) done_flag: Arc<AtomicBool>,
}

#[derive(Debug)]
pub(crate) struct ConnWait {
  mutex: Mutex<()>,
  value: AtomicUsize,
  await_cond: Condvar,
}

impl Default for ConnWait {
  fn default() -> Self {
    ConnWait { mutex: Mutex::new(()), value: AtomicUsize::new(0), await_cond: Condvar::new() }
  }
}

impl ConnWait {
  pub fn signal(&self, value: usize) {
    self.value.store(value, SeqCst);
    if let Ok(guard) = self.mutex.lock() {
      self.await_cond.notify_all();
      drop(guard);
    }
  }

  pub fn is_done(&self, value: usize) -> bool {
    self.value.load(SeqCst) >= value
  }

  fn wait_forever(&self, value: usize) -> bool {
    if self.is_done(value) {
      return true;
    }

    let Ok(mut guard) = self.mutex.lock() else {
      return false;
    };

    loop {
      if self.is_done(value) {
        return true;
      }
      guard = match self.await_cond.wait(guard) {
        Ok(guard) => guard,
        Err(_) => {
          return false;
        }
      }
    }
  }
  pub fn wait(&self, value: usize, timeout: Option<Duration>) -> bool {
    let Some(timeout) = timeout else {
      return self.wait_forever(value);
    };

    if self.is_done(value) {
      return true;
    }

    let Ok(mut guard) = self.mutex.lock() else {
      return false;
    };

    loop {
      if self.is_done(value) {
        return true;
      }
      guard = match self.await_cond.wait_timeout(guard, timeout) {
        Ok((guard, tm)) => {
          if tm.timed_out() {
            return false;
          }
          guard
        }
        Err(_) => {
          return false;
        }
      }
    }
  }
}
