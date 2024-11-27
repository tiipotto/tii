use crate::stream::{ConnectionStream, ConnectionStreamRead, ConnectionStreamWrite};
use crate::util::unwrap_poison;
use rust_tls_duplex_stream::RustTlsDuplexStream;
use rustls::server::ServerConnectionData;
use rustls::ServerConnection;
use std::fmt::Debug;
use std::io::{Read, Write};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};
use unowned_buf::{UnownedReadBuffer, UnownedWriteBuffer};

/// All connections that can be used to tunnel tls using Humpty's default RustTls wrapper need to provide these functions.
/// This trait is implemented by default for TcpStream and UnixStream.
pub trait TlsCapableStream: Debug + Sync + Send {
  /// io::Read &T
  fn read(&self, buf: &mut [u8]) -> io::Result<usize>;

  /// io::Write &T
  fn write(&self, buf: &[u8]) -> io::Result<usize>;

  /// io::Write &T
  fn flush(&self) -> io::Result<()>;

  /// This fn must cancel all concurrent read operations and prevent any new read+write operations from blocking.
  /// All ongoing and future operations are expected to return Err immediately after this fn was called.
  fn shutdown(&self);

  /// The address of the remote this stream.
  fn peer_addr(&self) -> io::Result<String>;
}

mod tcp {
  use crate::tls_stream::TlsCapableStream;
  use std::io;
  use std::net::{Shutdown, TcpStream};

  impl TlsCapableStream for TcpStream {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
      io::Read::read(&mut &*self, buf)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
      io::Write::write(&mut &*self, buf)
    }

    fn flush(&self) -> io::Result<()> {
      io::Write::flush(&mut &*self)
    }

    fn shutdown(&self) {
      _ = TcpStream::shutdown(self, Shutdown::Both);
    }

    fn peer_addr(&self) -> io::Result<String> {
      self.peer_addr().map(|p| p.to_string())
    }
  }
}

#[cfg(unix)]
mod unix {
  use crate::tls_stream::TlsCapableStream;
  use std::io;
  use std::net::Shutdown;
  use std::os::unix::net::UnixStream;

  impl TlsCapableStream for UnixStream {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
      io::Read::read(&mut &*self, buf)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
      io::Write::write(&mut &*self, buf)
    }

    fn flush(&self) -> io::Result<()> {
      io::Write::flush(&mut &*self)
    }

    fn shutdown(&self) {
      _ = UnixStream::shutdown(self, Shutdown::Both);
    }

    fn peer_addr(&self) -> io::Result<String> {
      self
        .peer_addr()
        .map(|p| p.as_pathname().map(|p| p.to_string_lossy().to_string()).unwrap_or_default())
    }
  }
}

#[derive(Debug)]
#[repr(transparent)]
struct StreamWrapper<T: TlsCapableStream + ?Sized>(Arc<T>);

impl<T: TlsCapableStream + ?Sized> Clone for StreamWrapper<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T: TlsCapableStream + ?Sized> Read for StreamWrapper<T> {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    self.0.deref().read(buf)
  }
}

impl<T: TlsCapableStream + ?Sized> Write for StreamWrapper<T> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.0.deref().write(buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    self.0.deref().flush()
  }
}

/// Wrapper struct that wraps a TLS Engine from RustTLS together with a read and write buffers.
#[derive(Debug, Clone)]
pub struct HumptyTlsStream(Arc<HumptyTlsWrapperInner>);
impl HumptyTlsStream {
  /// Create a new HumptyTlsStream using the given tcp stream.
  /// Calling this fn will create 2 background threads using `thread::Builder::new()::spawn`
  /// The threads are automatically stopped if the returned ConnectionStream is dropped.
  pub fn create_unpooled<S: TlsCapableStream + 'static>(
    tcp: S,
    tls: ServerConnection,
  ) -> io::Result<Box<dyn ConnectionStream>> {
    Self::create_tcp(tcp, tls, |task| thread::Builder::new().spawn(task).map(|_| {}))
  }

  /// Create a new HumptyTlsStream using the given tcp stream.
  /// Calling this fn will create 2 background threads using the provided thread spawn function.
  /// The tasks automatically return if the returned ConnectionStream is dropped.
  pub fn create_tcp<
    S: TlsCapableStream + 'static,
    T: FnMut(Box<dyn FnOnce() + Send>) -> io::Result<()>,
  >(
    stream: S,
    tls: ServerConnection,
    spawner: T,
  ) -> io::Result<Box<dyn ConnectionStream>> {
    let peer = stream.peer_addr()?.to_string();
    let stream_wrapper = StreamWrapper(Arc::new(stream));
    let tls =
      RustTlsDuplexStream::new(tls, stream_wrapper.clone(), stream_wrapper.clone(), spawner)?;

    Ok(Box::new(Self(Arc::new(HumptyTlsWrapperInner {
      stream_ref: stream_wrapper.0 as Arc<_>,
      tls,
      read: Mutex::new(UnownedReadBuffer::new()),
      write: Mutex::new(UnownedWriteBuffer::new()),
      peer,
    }))) as Box<dyn ConnectionStream>)
  }
}

#[derive(Debug)]
struct HumptyTlsWrapperInner {
  stream_ref: Arc<dyn TlsCapableStream>,
  tls: RustTlsDuplexStream<ServerConnection, ServerConnectionData>,
  read: Mutex<UnownedReadBuffer<0x4000>>,
  write: Mutex<UnownedWriteBuffer<0x4000>>,
  peer: String,
}

impl Drop for HumptyTlsWrapperInner {
  fn drop(&mut self) {
    self.stream_ref.shutdown()
  }
}

impl ConnectionStreamRead for HumptyTlsStream {
  fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
    unwrap_poison(self.0.read.lock())?.read(&mut &self.0.tls, buf)
  }

  fn ensure_readable(&self) -> io::Result<bool> {
    unwrap_poison(self.0.read.lock())?.ensure_readable(&mut &self.0.tls)
  }

  fn available(&self) -> usize {
    unwrap_poison(self.0.read.lock()).map(|g| g.available()).unwrap_or_default()
  }

  fn read_until(&self, end: u8, limit: usize, buf: &mut Vec<u8>) -> io::Result<usize> {
    unwrap_poison(self.0.read.lock())?.read_until_limit(&mut &self.0.tls, end, limit, buf)
  }

  fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
    unwrap_poison(self.0.read.lock())?.read_exact(&mut &self.0.tls, buf)
  }

  fn new_ref_read(&self) -> Box<dyn Read + Send + Sync> {
    Box::new(self.clone()) as Box<dyn Read + Send + Sync>
  }

  fn as_stream_read(&self) -> &dyn ConnectionStreamRead {
    self
  }

  fn new_ref_stream_read(&self) -> Box<dyn ConnectionStreamRead> {
    Box::new(self.clone()) as Box<dyn ConnectionStreamRead>
  }

  fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
    self.0.tls.set_read_timeout(dur)
  }

  fn get_read_timeout(&self) -> io::Result<Option<Duration>> {
    self.0.tls.read_timeout()
  }
}

impl Read for HumptyTlsStream {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    unwrap_poison(self.0.read.lock())?.read(&mut &self.0.tls, buf)
  }
}

impl ConnectionStreamWrite for HumptyTlsStream {
  fn write(&self, buf: &[u8]) -> io::Result<usize> {
    unwrap_poison(self.0.write.lock())?.write(&mut &self.0.tls, buf)
  }

  fn write_all(&self, buf: &[u8]) -> io::Result<()> {
    unwrap_poison(self.0.write.lock())?.write_all(&mut &self.0.tls, buf)
  }

  fn flush(&self) -> io::Result<()> {
    unwrap_poison(self.0.write.lock())?.flush(&mut &self.0.tls)
  }

  fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
    self.0.tls.set_write_timeout(dur)
  }

  fn get_write_timeout(&self) -> io::Result<Option<Duration>> {
    self.0.tls.write_timeout()
  }

  fn new_ref_write(&self) -> Box<dyn Write + Send + Sync> {
    Box::new(self.clone()) as Box<dyn Write + Send + Sync>
  }

  fn new_ref_stream_write(&self) -> Box<dyn ConnectionStreamWrite> {
    Box::new(self.clone()) as Box<dyn ConnectionStreamWrite>
  }

  fn as_stream_write(&self) -> &dyn ConnectionStreamWrite {
    self
  }
}

impl Write for HumptyTlsStream {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    unwrap_poison(self.0.write.lock())?.write(&mut &self.0.tls, buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    unwrap_poison(self.0.write.lock())?.flush(&mut &self.0.tls)
  }
}

impl ConnectionStream for HumptyTlsStream {
  fn new_ref(&self) -> Box<dyn ConnectionStream> {
    Box::new(self.clone()) as Box<dyn ConnectionStream>
  }

  fn peer_addr(&self) -> io::Result<String> {
    Ok(self.0.peer.clone())
  }
}
