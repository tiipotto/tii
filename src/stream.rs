//! Provides a wrapper around the stream to allow for simpler APIs.
//! TODO docs before release
#![allow(missing_docs)]

use std::fmt::Debug;
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

///
/// This represents a raw stream source the server can use to server requests to.
/// Each instance of this represents a dedicated client connection.
///
/// The Stream source is expected to be reference counted and handle concurrent reads/writes.
/// Separate concurrent calls to read and write must be possible independent of each other.
///
/// The implementation of this trait can assume that multiple concurrent calls to either read or write are not made.
///
/// How concurrent invocations of set_read_timeout/set_write_timeout are handled is implementation and platform specific.
/// Possible outcomes are:
/// - blocking read/write calls are canceled (fail with an error)
/// - set_read_timeout/set_write_timeout blocks until read/write calls are finished
/// - set_read_timeout/set_write_timeout only applies for future invocations of read/write and current invocations are left as is and will keep blocking.
///
///
pub trait ConnectionStream: ConnectionStreamRead + ConnectionStreamWrite {
  fn new_ref(&self) -> Box<dyn ConnectionStream>;

  fn peer_addr(&self) -> io::Result<String>;
  fn local_addr(&self) -> io::Result<String>;
}

pub trait ConnectionStreamRead: Sync + Send + Debug + Read {
  ///De-mut of Read
  fn read(&self, buf: &mut [u8]) -> io::Result<usize>;

  /// This fn returns true if at least 1 byte can be read.
  /// If the stream is EOF then false is returned.
  ///
  /// # Implementation Detail
  /// Caller can assume the following about this fn:
  /// This fn will call the underlying io::Read operation and buffer the output of read unless it already has buffered data previously.
  /// The next call to any reading function is expected to return data from the internal buffer instead of calling the underlying io::Read operation.
  ///
  /// # Errors
  /// TimedOut/WouldBlock indicates that a timeout would have occurred when reading 1 byte.
  /// Other errors that would have occurred when calling the underlying io operation.
  ///
  fn ensure_readable(&self) -> io::Result<bool>;

  /// Returns the amount of bytes available for reading without blocking or errors.
  /// Caller can assume with high likelihood that a call read_exact with the returned number of bytes or less
  /// will not error or block
  fn available(&self) -> usize;

  ///De-mut of BufReader
  fn read_until(&self, end: u8, limit: usize, buf: &mut Vec<u8>) -> io::Result<usize>;

  ///De-mut of Read
  fn read_exact(&self, buf: &mut [u8]) -> io::Result<()>;

  fn new_ref_read(&self) -> Box<dyn Read + Send + Sync>;

  fn as_stream_read(&self) -> &dyn ConnectionStreamRead;

  fn new_ref_stream_read(&self) -> Box<dyn ConnectionStreamRead>;

  fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

  fn get_read_timeout(&self) -> io::Result<Option<Duration>>;
}

pub trait ConnectionStreamWrite: Sync + Send + Debug + Write {
  ///De-mut of Write
  fn write(&self, buf: &[u8]) -> io::Result<usize>;
  ///De-mut of Write
  fn write_all(&self, buf: &[u8]) -> io::Result<()>;

  ///De-mut of Write
  fn flush(&self) -> io::Result<()>;

  fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

  fn get_write_timeout(&self) -> io::Result<Option<Duration>>;

  fn new_ref_write(&self) -> Box<dyn Write + Send + Sync>;

  fn new_ref_stream_write(&self) -> Box<dyn ConnectionStreamWrite>;
  fn as_stream_write(&self) -> &dyn ConnectionStreamWrite;
}

pub trait IntoConnectionStream {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream>;
}

impl IntoConnectionStream for TcpStream {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream> {
    tcp::new(self)
  }
}

impl IntoConnectionStream for Box<dyn ConnectionStream> {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream> {
    self
  }
}

impl IntoConnectionStream for (Box<dyn Read + Send>, Box<dyn Write + Send>) {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream> {
    boxed::new(self.0, self.1)
  }
}

mod tcp {
  use crate::stream::{ConnectionStream, ConnectionStreamRead, ConnectionStreamWrite};
  use crate::util::unwrap_poison;
  use std::fmt::Debug;
  use std::io;
  use std::io::{Read, Write};
  use std::net::TcpStream;
  use std::sync::{Arc, Mutex};
  use std::time::Duration;
  use unowned_buf::{UnownedReadBuffer, UnownedWriteBuffer};

  pub fn new(stream: TcpStream) -> Box<dyn ConnectionStream> {
    Box::new(TcpStreamOuter(Arc::new(TcpStreamInner::new(stream))))
  }

  #[derive(Debug, Clone)]
  struct TcpStreamOuter(Arc<TcpStreamInner>);

  #[derive(Debug)]
  struct TcpStreamInner {
    read_mutex: Mutex<UnownedReadBuffer<0x4000>>,
    write_mutex: Mutex<UnownedWriteBuffer<0x4000>>,
    stream: TcpStream,
  }
  impl TcpStreamInner {
    fn new(stream: TcpStream) -> TcpStreamInner {
      TcpStreamInner {
        read_mutex: Mutex::new(UnownedReadBuffer::new()),
        write_mutex: Mutex::new(UnownedWriteBuffer::new()),
        stream,
      }
    }
  }

  impl Read for TcpStreamOuter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
      ConnectionStreamRead::read(self, buf)
    }
  }

  impl ConnectionStreamRead for TcpStreamOuter {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
      unwrap_poison(self.0.read_mutex.lock())?.read(&mut &self.0.stream, buf)
    }

    fn ensure_readable(&self) -> io::Result<bool> {
      unwrap_poison(self.0.read_mutex.lock())?.ensure_readable(&mut &self.0.stream)
    }

    fn available(&self) -> usize {
      // if we are poisoned, we for sure cant read anything!
      unwrap_poison(self.0.read_mutex.lock()).map(|g| g.available()).unwrap_or_default()
    }

    fn read_until(&self, end: u8, limit: usize, buf: &mut Vec<u8>) -> io::Result<usize> {
      unwrap_poison(self.0.read_mutex.lock())?.read_until_limit(
        &mut &self.0.stream,
        end,
        limit,
        buf,
      )
    }

    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
      unwrap_poison(self.0.read_mutex.lock())?.read_exact(&mut &self.0.stream, buf)
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
      self.0.stream.set_read_timeout(dur)
    }

    fn get_read_timeout(&self) -> io::Result<Option<Duration>> {
      self.0.stream.read_timeout()
    }
  }

  impl ConnectionStreamWrite for TcpStreamOuter {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
      unwrap_poison(self.0.write_mutex.lock())?.write(&mut &self.0.stream, buf)
    }

    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
      unwrap_poison(self.0.write_mutex.lock())?.write_all(&mut &self.0.stream, buf)
    }

    fn flush(&self) -> io::Result<()> {
      unwrap_poison(self.0.write_mutex.lock())?.flush(&mut &self.0.stream)
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
      self.0.stream.set_write_timeout(dur)
    }

    fn get_write_timeout(&self) -> io::Result<Option<Duration>> {
      self.0.stream.write_timeout()
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

  impl Write for TcpStreamOuter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      ConnectionStreamWrite::write(self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
      ConnectionStreamWrite::flush(self)
    }
  }

  impl ConnectionStream for TcpStreamOuter {
    fn new_ref(&self) -> Box<dyn ConnectionStream> {
      Box::new(self.clone()) as Box<dyn ConnectionStream>
    }

    fn peer_addr(&self) -> io::Result<String> {
      Ok(format!("{}", self.0.stream.peer_addr()?))
    }

    fn local_addr(&self) -> io::Result<String> {
      Ok(format!("{}", self.0.stream.local_addr()?))
    }
  }
}

//TODO what about timeout?
mod boxed {
  use crate::stream::{ConnectionStream, ConnectionStreamRead, ConnectionStreamWrite};
  use crate::util::unwrap_poison;
  use std::fmt::{Debug, Formatter};
  use std::io;
  use std::io::{BufWriter, Read, Write};
  use std::ops::DerefMut;
  use std::sync::{Arc, Mutex};
  use std::time::Duration;
  use unowned_buf::UnownedReadBuffer;

  pub fn new(
    read: Box<dyn Read + Send>,
    write: Box<dyn Write + Send>,
  ) -> Box<dyn ConnectionStream> {
    Box::new(BoxStreamOuter(Arc::new(BoxStreamInner {
      read_mutex: Mutex::new((UnownedReadBuffer::default(), read)),
      write_mutex: Mutex::new(BufWriter::new(write)),
    }))) as Box<dyn ConnectionStream>
  }

  #[derive(Debug, Clone)]
  struct BoxStreamOuter(Arc<BoxStreamInner>);

  struct BoxStreamInner {
    read_mutex: Mutex<(UnownedReadBuffer<0x4000>, Box<dyn Read + Send>)>,
    write_mutex: Mutex<BufWriter<Box<dyn Write + Send>>>,
  }

  impl Debug for BoxStreamInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      f.write_str("BoxStreamInner")
    }
  }

  impl ConnectionStreamRead for BoxStreamOuter {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
      let mut guard = unwrap_poison(self.0.read_mutex.lock())?;
      let (buffer, stream) = guard.deref_mut();
      buffer.read(stream, buf)
    }

    fn ensure_readable(&self) -> io::Result<bool> {
      let mut guard = unwrap_poison(self.0.read_mutex.lock())?;
      let (buffer, stream) = guard.deref_mut();
      buffer.ensure_readable(stream)
    }

    fn available(&self) -> usize {
      unwrap_poison(self.0.read_mutex.lock()).map(|g| g.0.available()).unwrap_or_default()
    }

    fn read_until(&self, end: u8, limit: usize, buf: &mut Vec<u8>) -> io::Result<usize> {
      let mut guard = unwrap_poison(self.0.read_mutex.lock())?;
      let (buffer, stream) = guard.deref_mut();
      buffer.read_until_limit(stream, end, limit, buf)
    }

    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
      let mut guard = unwrap_poison(self.0.read_mutex.lock())?;
      let (buffer, stream) = guard.deref_mut();
      buffer.read_exact(stream, buf)
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

    fn set_read_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
      Ok(())
    }

    fn get_read_timeout(&self) -> io::Result<Option<Duration>> {
      Ok(None)
    }
  }

  impl Read for BoxStreamOuter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
      ConnectionStreamRead::read(self, buf)
    }
  }

  impl ConnectionStreamWrite for BoxStreamOuter {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
      unwrap_poison(self.0.write_mutex.lock())?.write(buf)
    }

    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
      unwrap_poison(self.0.write_mutex.lock())?.write_all(buf)
    }

    fn flush(&self) -> std::io::Result<()> {
      unwrap_poison(self.0.write_mutex.lock())?.flush()
    }

    fn set_write_timeout(&self, _dur: Option<Duration>) -> io::Result<()> {
      Ok(())
    }

    fn get_write_timeout(&self) -> io::Result<Option<Duration>> {
      Ok(None)
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

  impl io::Write for BoxStreamOuter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      ConnectionStreamWrite::write(self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
      ConnectionStreamWrite::flush(self)
    }
  }

  impl ConnectionStream for BoxStreamOuter {
    fn new_ref(&self) -> Box<dyn ConnectionStream> {
      Box::new(self.clone()) as Box<dyn ConnectionStream>
    }

    fn peer_addr(&self) -> io::Result<String> {
      Ok("Box".to_string())
    }

    fn local_addr(&self) -> io::Result<String> {
      Ok("Box".to_string())
    }
  }
}

#[cfg(unix)]
impl IntoConnectionStream for std::os::unix::net::UnixStream {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream> {
    unix::new(self)
  }
}

#[cfg(unix)]
mod unix {
  use crate::stream::{ConnectionStream, ConnectionStreamRead, ConnectionStreamWrite};
  use crate::util::unwrap_poison;
  use std::fmt::Debug;
  use std::io;
  use std::io::{Read, Write};
  use std::os::unix::net::UnixStream;
  use std::sync::{Arc, Mutex};
  use std::time::Duration;
  use unowned_buf::{UnownedReadBuffer, UnownedWriteBuffer};

  pub fn new(stream: UnixStream) -> Box<dyn ConnectionStream> {
    Box::new(UnixStreamOuter(Arc::new(UnixStreamInner::new(stream))))
  }

  #[derive(Debug, Clone)]
  struct UnixStreamOuter(Arc<UnixStreamInner>);

  #[derive(Debug)]
  struct UnixStreamInner {
    read_mutex: Mutex<UnownedReadBuffer<0x4000>>,
    write_mutex: Mutex<UnownedWriteBuffer<0x4000>>,
    stream: UnixStream,
  }

  impl UnixStreamInner {
    fn new(stream: UnixStream) -> UnixStreamInner {
      UnixStreamInner {
        read_mutex: Mutex::new(UnownedReadBuffer::new()),
        write_mutex: Mutex::new(UnownedWriteBuffer::new()),
        stream,
      }
    }
  }

  impl Read for UnixStreamOuter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
      ConnectionStreamRead::read(self, buf)
    }
  }

  impl ConnectionStreamRead for UnixStreamOuter {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
      unwrap_poison(self.0.read_mutex.lock())?.read(&mut &self.0.stream, buf)
    }

    fn available(&self) -> usize {
      // if we are poisoned, we for sure cant read anything!
      unwrap_poison(self.0.read_mutex.lock()).map(|g| g.available()).unwrap_or_default()
    }

    fn ensure_readable(&self) -> io::Result<bool> {
      unwrap_poison(self.0.read_mutex.lock())?.ensure_readable(&mut &self.0.stream)
    }

    fn read_until(&self, end: u8, limit: usize, buf: &mut Vec<u8>) -> io::Result<usize> {
      unwrap_poison(self.0.read_mutex.lock())?.read_until_limit(
        &mut &self.0.stream,
        end,
        limit,
        buf,
      )
    }

    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
      unwrap_poison(self.0.read_mutex.lock())?.read_exact(&mut &self.0.stream, buf)
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
      self.0.stream.set_read_timeout(dur)
    }

    fn get_read_timeout(&self) -> io::Result<Option<Duration>> {
      self.0.stream.read_timeout()
    }
  }

  impl ConnectionStreamWrite for UnixStreamOuter {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
      unwrap_poison(self.0.write_mutex.lock())?.write(&mut &self.0.stream, buf)
    }

    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
      unwrap_poison(self.0.write_mutex.lock())?.write_all(&mut &self.0.stream, buf)
    }

    fn flush(&self) -> io::Result<()> {
      unwrap_poison(self.0.write_mutex.lock())?.flush(&mut &self.0.stream)
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
      self.0.stream.set_write_timeout(dur)
    }

    fn get_write_timeout(&self) -> io::Result<Option<Duration>> {
      self.0.stream.write_timeout()
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

  impl Write for UnixStreamOuter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      ConnectionStreamWrite::write(self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
      ConnectionStreamWrite::flush(self)
    }
  }

  impl ConnectionStream for UnixStreamOuter {
    fn new_ref(&self) -> Box<dyn ConnectionStream> {
      Box::new(self.clone()) as Box<dyn ConnectionStream>
    }

    fn peer_addr(&self) -> io::Result<String> {
      Ok("unix".to_string())
    }

    fn local_addr(&self) -> io::Result<String> {
      self
        .0
        .stream
        .local_addr()
        .map(|a| a.as_pathname().map(|a| a.to_string_lossy().to_string()).unwrap_or_default())
    }
  }
}
