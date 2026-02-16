use crate::extras::tls_connector_impl::{Adapter, TlsConnectorImpl};
use crate::extras::{Connector, ConnectorMeta, CONNECTOR_SHUTDOWN_FLAG_POLLING_INTERVAL};
use crate::functional_traits::DefaultThreadAdapter;
use crate::{ConnectionStream, Server, ThreadAdapter, TiiResult, TlsStream};
use listener_poll::PollEx;
use rustls::{ServerConfig, ServerConnection};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Handle to a listening TCP Server Socket that waits for incoming TLS connections and then passing them on to Tii.
#[derive(Debug)]
pub struct TlsTcpConnector(TlsConnectorImpl<TcpListener, TcpStream, TlsTcpConnector>);

impl TlsTcpConnector {
  /// Creates a new TlsTcpConnector that is listening on the given addr.
  /// Return Err on error.
  /// The TCP listener will listen immediately in a background thread.
  pub fn start(
    addr: impl ToSocketAddrs,
    tii_server: Arc<Server>,
    config: Arc<ServerConfig>,
    thread_adapter: impl ThreadAdapter + 'static,
    permit_plain_text: bool,
  ) -> TiiResult<Self> {
    let mut addr_string = String::new();
    let addr_in_vec = addr.to_socket_addrs()?.collect::<Vec<SocketAddr>>();

    for addr in &addr_in_vec {
      if !addr_string.is_empty() {
        addr_string += ", ";
      }
      addr_string += addr.to_string().as_str();
    }

    let listener = TcpListener::bind(addr)?;

    TlsConnectorImpl::start(
      "tls_tcp_connector",
      addr_string,
      listener,
      tii_server,
      config,
      thread_adapter,
      permit_plain_text,
    )
    .map(Self)
  }

  /// Create a new TlsTcpConnector.
  /// When this fn returns Ok() the socket is already listening in a background thread.
  /// Returns an io::Error if it was unable to bind to the socket.
  ///
  /// Threads are created using "thread::Builder::new().spawn"
  pub fn start_unpooled(
    addr: impl ToSocketAddrs,
    config: Arc<ServerConfig>,
    tii_server: Arc<Server>,
    permit_plain_text: bool,
  ) -> TiiResult<Self> {
    Self::start(addr, tii_server, config, DefaultThreadAdapter, permit_plain_text)
  }
}
impl Adapter<TcpListener, TcpStream> for TlsTcpConnector {
  fn listener_set_nonblocking(listener: &TcpListener, flag: bool) -> io::Result<()> {
    listener.set_nonblocking(flag)
  }

  fn stream_set_nonblocking(listener: &TcpStream, flag: bool) -> io::Result<()> {
    listener.set_nonblocking(flag)
  }

  fn set_read_timeout(stream: &TcpStream, timeout: Option<Duration>) -> io::Result<()> {
    stream.set_read_timeout(timeout)
  }

  fn read_exact(mut stream: &TcpStream, buf: &mut [u8]) -> io::Result<()> {
    io::Read::read_exact(&mut stream, buf)
  }

  fn accept(
    listener: &TcpListener,
    tii: &Server,
    shutdown_flag: &AtomicBool,
  ) -> io::Result<TcpStream> {
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
    stream: TcpStream,
    initial_data: &[u8],
    scon: ServerConnection,
    spawner: &dyn ThreadAdapter,
  ) -> io::Result<Box<dyn ConnectionStream>> {
    TlsStream::create_with_initial_data(stream, initial_data, scon, spawner)
  }

  fn plain(stream: TcpStream, initial_data: &[u8]) -> Box<dyn ConnectionStream> {
    crate::stream::tcp_stream_new(stream, initial_data)
  }

  fn meta_tls() -> ConnectorMeta {
    ConnectorMeta::TlsTcp
  }

  fn meta_plain() -> ConnectorMeta {
    ConnectorMeta::Tcp
  }
}

impl Connector for TlsTcpConnector {
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
