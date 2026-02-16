use crate::extras::tls_connector_impl::{Adapter, TlsConnectorImpl};
use crate::extras::{Connector, ConnectorMeta, CONNECTOR_SHUTDOWN_FLAG_POLLING_INTERVAL};
use crate::functional_traits::DefaultThreadAdapter;
use crate::{ConnectionStream, Server, ThreadAdapter, TiiResult, TlsStream};
use listener_poll::PollEx;
use rustls::{ServerConfig, ServerConnection};
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Handle to a listening Unix socket that waits for incoming TLS connections and then passing them on to Tii.
#[derive(Debug)]
pub struct TlsUnixConnector(TlsConnectorImpl<UnixListener, UnixStream, TlsUnixConnector>);

impl TlsUnixConnector {
  /// Creates a new TlsUnixConnector that is listening on the given addr.
  /// Return Err on error.
  /// The UnixListener will listen immediately in a background thread.
  pub fn start(
    addr: impl AsRef<Path>,
    tii_server: Arc<Server>,
    config: Arc<ServerConfig>,
    thread_adapter: impl ThreadAdapter + 'static,
    permit_plain_text: bool,
  ) -> TiiResult<Self> {
    let path = addr.as_ref();
    if std::fs::exists(path)? {
      std::fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;

    TlsConnectorImpl::start(
      "tls_unix_connector",
      path.display().to_string(),
      listener,
      tii_server,
      config,
      thread_adapter,
      permit_plain_text,
    )
    .map(Self)
  }

  /// Create a new TlsUnixConnector.
  /// When this fn returns Ok() the socket is already listening in a background thread.
  /// Returns an io::Error if it was unable to bind to the socket.
  ///
  /// Threads are created using "thread::Builder::new().spawn"
  pub fn start_unpooled(
    addr: impl AsRef<Path>,
    config: Arc<ServerConfig>,
    tii_server: Arc<Server>,
    permit_plain_text: bool,
  ) -> TiiResult<Self> {
    Self::start(addr, tii_server, config, DefaultThreadAdapter, permit_plain_text)
  }
}
impl Adapter<UnixListener, UnixStream> for TlsUnixConnector {
  fn listener_set_nonblocking(listener: &UnixListener, flag: bool) -> std::io::Result<()> {
    listener.set_nonblocking(flag)
  }

  fn stream_set_nonblocking(listener: &UnixStream, flag: bool) -> std::io::Result<()> {
    listener.set_nonblocking(flag)
  }

  fn set_read_timeout(stream: &UnixStream, timeout: Option<Duration>) -> std::io::Result<()> {
    stream.set_read_timeout(timeout)
  }

  fn read_exact(mut stream: &UnixStream, buf: &mut [u8]) -> std::io::Result<()> {
    std::io::Read::read_exact(&mut stream, buf)
  }

  fn accept(
    listener: &UnixListener,
    tii: &Server,
    shutdown_flag: &AtomicBool,
  ) -> io::Result<UnixStream> {
    loop {
      if tii.is_shutdown() || shutdown_flag.load(Ordering::SeqCst) {
        return Err(io::ErrorKind::ConnectionAborted.into());
      }

      if !listener.poll(Some(CONNECTOR_SHUTDOWN_FLAG_POLLING_INTERVAL))? {
        continue;
      }

      return listener.accept().map(|(stream, _)| stream);
    }
  }

  fn tls(
    stream: UnixStream,
    initial_data: &[u8],
    scon: ServerConnection,
    spawner: &dyn ThreadAdapter,
  ) -> io::Result<Box<dyn ConnectionStream>> {
    TlsStream::create_with_initial_data(stream, initial_data, scon, spawner)
  }

  fn plain(stream: UnixStream, initial_data: &[u8]) -> Box<dyn ConnectionStream> {
    crate::stream::unix_stream_new(stream, initial_data)
  }

  fn meta_tls() -> ConnectorMeta {
    ConnectorMeta::TlsUnix
  }

  fn meta_plain() -> ConnectorMeta {
    ConnectorMeta::Unix
  }
}

impl Connector for TlsUnixConnector {
  fn shutdown(&self) {
    self.0.shutdown();
  }

  fn is_marked_for_shutdown(&self) -> bool {
    self.0.is_marked_for_shutdown()
  }

  fn is_shutting_down(&self) -> bool {
    self.0.is_shutting_down()
  }

  fn is_shutdown(&self) -> bool {
    self.0.is_shutdown()
  }

  fn shutdown_and_join(&self, timeout: Option<Duration>) -> bool {
    self.0.shutdown_and_join(timeout)
  }

  fn join(&self, timeout: Option<Duration>) -> bool {
    self.0.join(timeout)
  }
}
