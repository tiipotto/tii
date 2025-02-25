//! Provides functionality for http request bodies

use crate::util::{unwrap_poison, unwrap_some};
use crate::{error_log, TiiResult};
use libflate::gzip;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{Cursor, Error, ErrorKind, Read, Take};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

/// Request body abstraction that will implement a generic read/write interface that does not require mut self to operate upon and can be shared with threads.
/// Peculiarities like transfer and content encoding are handled by the inner implementation and hidden from the actual endpoints.
///
/// # Concerning equality
/// Instances of RequestBody are only considered equal if they refer to the exact same underlying stream.
/// The content of the stream is not considered when determining equality as that would require consuming the stream.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct RequestBody(Arc<Mutex<RequestBodyInner>>);

impl Eq for RequestBody {}
impl PartialEq for RequestBody {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl RequestBody {

  /// For unit tests or mocks, will mimic new_with_content_length
  /// This will call to_vec() on the slice.
  pub fn new_with_data_ref<T: AsRef<[u8]>>(data: T) -> RequestBody {
    Self::new_with_data(data.as_ref().to_vec())
  }

  /// For unit tests or mocks, will mimic new_with_content_length
  pub fn new_with_data(data: Vec<u8>) -> RequestBody {
    let len = data.len() as u64;
    let cursor = Cursor::new(data);
    Self::new_with_content_length(Box::new(cursor) as Box<dyn Read + Send + 'static>, len)
  }

  /// Uncompressed stream with known length.
  pub fn new_with_content_length<T: Read + Send + 'static>(read: T, len: u64) -> RequestBody {
    RequestBody(Arc::new(Mutex::new(RequestBodyInner::WithContentLength(
      RequestBodyWithContentLength {
        err: false,
        data: (Box::new(read) as Box<dyn Read + Send>).take(len),
      },
    ))))
  }

  /// Uncompressed Chunked stream. Content length is not known.
  pub fn new_chunked<T: Read + Send + 'static>(read: T) -> RequestBody {
    RequestBody(Arc::new(Mutex::new(RequestBodyInner::Chunked(RequestBodyChunked {
      read: Box::new(read) as Box<dyn Read + Send>,
      eof: false,
      err: false,
      remaining_chunk_length: 0,
    }))))
  }

  /// Chunked stream that is gzip compressed.
  /// Neither compressed nor uncompressed content lengths are known.
  pub fn new_gzip_chunked<T: Read + Send + 'static>(read: T) -> TiiResult<RequestBody> {
    let inner = RequestBodyInner::Chunked(RequestBodyChunked {
      read: Box::new(read) as Box<dyn Read + Send>,
      eof: false,
      err: false,
      remaining_chunk_length: 0,
    });

    Ok(RequestBody(Arc::new(Mutex::new(RequestBodyInner::Gzip(GzipRequestBody::new(inner)?)))))
  }

  /// GZIP stream with a known length of the uncompressed data.
  /// The size of the gzip payload is presumably smaller (not guaranteed) but otherwise unknown.
  pub fn new_gzip_with_uncompressed_length<T: Read + Send + 'static>(
    read: T,
    len: u64,
  ) -> TiiResult<RequestBody> {
    let decoder = gzip::Decoder::new(read).inspect_err(|e| {
      error_log!("Could not decode gzip header of request body: {}", e);
    })?;

    Ok(Self::new_with_content_length(decoder, len))
  }

  /// GZIP stream with a known length of the compressed data.
  /// The length of the uncompressed data is not known.
  pub fn new_gzip_with_compressed_content_length<T: Read + Send + 'static>(
    read: T,
    len: u64,
  ) -> TiiResult<RequestBody> {
    let inner = RequestBodyInner::WithContentLength(RequestBodyWithContentLength {
      err: false,
      data: (Box::new(read) as Box<dyn Read + Send>).take(len),
    });

    Ok(RequestBody(Arc::new(Mutex::new(RequestBodyInner::Gzip(GzipRequestBody::new(inner)?)))))
  }
}

impl RequestBody {

  /// Turns this struct into a generic Box<Read> impl.
  /// Useful for calling some external library functions.
  pub fn as_read(&self) -> impl Read + '_ {
    Box::new(self)
  }

  /// same as `std::io::Read` trait
  pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
    unwrap_poison(self.0.lock())?.deref_mut().read(buf)
  }

  /// same as `std::io::Read` trait
  pub fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
    unwrap_poison(self.0.lock())?.deref_mut().read_to_end(buf)
  }

  /// same as `std::io::Read` trait
  pub fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
    unwrap_poison(self.0.lock())?.deref_mut().read_exact(buf)
  }

  /// Returns the amount of bytes that can be read until the request body is fully consumes.
  /// Some types of request bodies do not know this in this case None is returned.
  /// # Errors
  /// If the read mutex was poisoned.
  pub fn remaining(&self) -> io::Result<Option<u64>> {
    Ok(match unwrap_poison(self.0.lock())?.deref_mut() {
      RequestBodyInner::WithContentLength(wc) => Some(wc.data.limit()),
      _ => None,
    })
  }
}

impl Read for &RequestBody {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    RequestBody::read(self, buf) //de-mut upcall
  }
}

#[derive(Debug)]
enum RequestBodyInner {
  WithContentLength(RequestBodyWithContentLength),
  Chunked(RequestBodyChunked),
  Gzip(GzipRequestBody),
  //Gzipped(...)   //..
}

impl Read for RequestBodyInner {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    match self {
      RequestBodyInner::WithContentLength(body) => body.read(buf),
      RequestBodyInner::Chunked(body) => body.read(buf),
      RequestBodyInner::Gzip(body) => body.read(buf),
    }
  }
}

#[derive(Debug)]
#[repr(transparent)]
struct GzipRequestBody(gzip::Decoder<Box<RequestBodyInner>>);

impl GzipRequestBody {
  fn new(inner: RequestBodyInner) -> TiiResult<Self> {
    let decoder = gzip::Decoder::new(Box::new(inner)).inspect_err(|e| {
      error_log!("Could not decode gzip header of request body: {}", e);
    })?;
    Ok(Self(decoder))
  }
}

impl Read for GzipRequestBody {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    self.0.read(buf)
  }
}

struct RequestBodyWithContentLength {
  err: bool,
  data: Take<Box<dyn Read + Send>>,
}

impl Read for RequestBodyWithContentLength {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    if self.err {
      return Err(Error::new(
        ErrorKind::BrokenPipe,
        "Transfer stream has failed due to previous error",
      ));
    }
    self.data.read(buf).inspect_err(|_| self.err = true)
  }
}

impl Debug for RequestBodyWithContentLength {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("RequestBodyWithContentLength(remaining={})", self.data.limit()))
  }
}

struct RequestBodyChunked {
  read: Box<dyn Read + Send>,
  eof: bool,
  err: bool,
  remaining_chunk_length: u64,
}

impl Debug for RequestBodyChunked {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!(
      "RequestBodyChunked(eof={} remaining_chunk_length={})",
      self.eof, self.remaining_chunk_length
    ))
  }
}

impl RequestBodyChunked {
  #[expect(clippy::indexing_slicing, reason = "we break if n >= 17")]
  fn read_internal(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    if buf.is_empty() {
      return Ok(0);
    }

    if self.eof {
      return Ok(0);
    }

    if self.remaining_chunk_length > 0 {
      let to_read = u64::min(buf.len() as u64, self.remaining_chunk_length) as usize;
      let read = self.read.read(&mut buf[..to_read])?;
      if read == 0 {
        return Err(Error::new(
          ErrorKind::UnexpectedEof,
          "chunked transfer encoding suggest more data",
        ));
      }

      self.remaining_chunk_length =
        unwrap_some(self.remaining_chunk_length.checked_sub(read as u64));
      if self.remaining_chunk_length == 0 {
        let mut tiny_buffer = [0u8; 1];
        self.read.read_exact(&mut tiny_buffer)?;
        if tiny_buffer[0] != b'\r' {
          return Err(Error::new(io::ErrorKind::InvalidData, "Chunk trailer is malformed"));
        }
        self.read.read_exact(&mut tiny_buffer)?;
        if tiny_buffer[0] != b'\n' {
          return Err(Error::new(io::ErrorKind::InvalidData, "Chunk trailer is malformed"));
        }
      }
      return Ok(read);
    }

    let mut small_buffer = [0u8; 32];
    let mut n = 0;
    loop {
      if n >= 17 {
        //If the client prefixes the chunks with '0' characters then we just don't support that.
        return Err(Error::new(
          io::ErrorKind::InvalidData,
          "Chunk size is larger than 2^64 or malformed",
        ));
      }
      self.read.read_exact(&mut small_buffer[n..n + 1])?;
      if small_buffer[n] == b'\r' {
        self.read.read_exact(&mut small_buffer[n..n + 1])?;
        if small_buffer[n] != b'\n' {
          return Err(Error::new(io::ErrorKind::InvalidData, "Chunk size is malformed"));
        }
        break;
      }

      n += 1;
    }

    if n == 0 {
      return Err(Error::new(io::ErrorKind::InvalidData, "Chunk size is malformed"));
    }

    let str = std::str::from_utf8(&small_buffer[0..n])
      .map_err(|_| Error::new(io::ErrorKind::InvalidData, "Chunk size is malformed"))?;
    let chunk_len = u64::from_str_radix(str, 16)
      .map_err(|_| Error::new(io::ErrorKind::InvalidData, "Chunk size is malformed"))?;
    if chunk_len == 0 {
      self.read.read_exact(&mut small_buffer[n..n + 1])?;
      if small_buffer[n] != b'\r' {
        return Err(Error::new(io::ErrorKind::InvalidData, "Chunk trailer is malformed"));
      }
      self.read.read_exact(&mut small_buffer[n..n + 1])?;
      if small_buffer[n] != b'\n' {
        return Err(Error::new(io::ErrorKind::InvalidData, "Chunk trailer is malformed"));
      }

      self.eof = true;
      return Ok(0);
    }

    self.remaining_chunk_length = chunk_len;
    self.read(buf)
  }
}

impl Read for RequestBodyChunked {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    if self.err {
      return Err(Error::new(
        ErrorKind::BrokenPipe,
        "Chunked transfer stream has failed due to previous error",
      ));
    }
    self.read_internal(buf).inspect_err(|_| self.err = true)
  }
}
