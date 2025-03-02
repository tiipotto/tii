//! Provides functionality for http response bodies
//! TODO docs before release
#![allow(missing_docs)]

use crate::stream::ConnectionStreamWrite;
use crate::util::unwrap_some;
use libflate::gzip;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};

pub(crate) type ResponseBodyHandler = dyn FnOnce(&dyn ResponseBodySink) -> io::Result<()>;

#[repr(transparent)]
#[derive(Debug)]
pub struct ResponseBody(ResponseBodyInner);

// We don't want to expose this enum.
enum ResponseBodyInner {
  //Fixed length data, content length header will be set automatically
  FixedSizeBinaryData(Vec<u8>),

  //Fixed length data, content length header will be set automatically
  FixedSizeTextData(String),

  //Streams a file.
  //Content length header will be set automatically
  FixedSizeFile(Box<dyn ReadAndSeek>, u64),

  //Content length header will not be set.
  //This forces Connection-Close after the request has been processed.
  //The caused overhead is that the client has to redo the connection.
  //This is acceptable for sending very large streaming data.
  Stream(Option<Box<ResponseBodyHandler>>),

  //Causes the response to be sent as chunked transfer encoding
  //All required headers for this will be set automatically.
  ChunkedStream(Option<Box<ResponseBodyHandler>>),

  //Data that has been externally gzipped. The Vec contains data in gzip format
  ExternallyGzippedData(Vec<u8>),

  //A file that has been externally gzipped. The files binary content is in gzip format
  ExternallyGzippedFile(Box<dyn ReadAndSeek>, u64),

  //Chunked Stream that will be gzipped on the fly
  ChunkedGzipStream(Option<Box<ResponseBodyHandler>>),

  //File that will be gzipped on the fly and sent in chunks
  ChunkedGzipFile(Box<dyn ReadAndSeek>),
}

impl Debug for ResponseBodyInner {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      ResponseBodyInner::FixedSizeBinaryData(data) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeBinaryData({:?})", data))
      }
      ResponseBodyInner::FixedSizeTextData(data) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeTextData({:?})", data))
      }
      ResponseBodyInner::FixedSizeFile(_, size) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeFile(file, {})", size))
      }
      ResponseBodyInner::Stream(_) => f.write_str("ResponseBody::Stream(...)"),
      ResponseBodyInner::ChunkedStream(_) => f.write_str("ResponseBody::ChunkedStream(...)"),
      ResponseBodyInner::ExternallyGzippedData(_) => {
        f.write_str("ResponseBody::ExternallyGzippedData(...)")
      }
      ResponseBodyInner::ExternallyGzippedFile(_, _) => {
        f.write_str("ResponseBody::ExternallyGzippedFile(...)")
      }
      ResponseBodyInner::ChunkedGzipStream(_) => {
        f.write_str("ResponseBody::ChunkedGzipStream(...)")
      }
      ResponseBodyInner::ChunkedGzipFile(_) => f.write_str("ResponseBody::ChunkedGzipFile(...)"),
    }
  }
}

// we don't really exactly need file. We need read+seek.
pub(crate) trait ReadAndSeek: Read + Seek {}

impl<T> ReadAndSeek for T where T: Read + Seek {}

pub trait ResponseBodySink {
  fn write(&self, buffer: &[u8]) -> io::Result<usize>;
  fn write_all(&self, buffer: &[u8]) -> io::Result<()>;

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_>;
}
impl ResponseBody {
  pub fn from_data(data: Vec<u8>) -> Self {
    Self(ResponseBodyInner::FixedSizeBinaryData(data))
  }

  /// Will send data that has been externally gzipped. the data is assumed to be in gzip format and this is not checked.
  pub fn from_externally_gzipped_data(data_in_gzip_format: Vec<u8>) -> Self {
    Self(ResponseBodyInner::ExternallyGzippedData(data_in_gzip_format))
  }

  /// Will gzip the data in memory and then send the compressed version of the data.
  pub fn from_data_with_gzip_in_memory(data: impl AsRef<[u8]>) -> Self {
    let data = data.as_ref();
    //We don't do any IO here, this should be infallible unless we run out of memory to enlarge the Vec in which case we might as well die.
    let mut encoder =
      crate::util::unwrap_ok(crate::util::new_gzip_encoder(Vec::with_capacity(data.len() + 128)));
    crate::util::unwrap_ok(encoder.write_all(data));
    let buffer = crate::util::unwrap_ok(encoder.finish().into_result());
    Self(ResponseBodyInner::ExternallyGzippedData(buffer))
  }

  pub fn from_string(data: String) -> Self {
    Self(ResponseBodyInner::FixedSizeTextData(data))
  }

  pub fn from_slice<T: AsRef<[u8]> + ?Sized>(data: &T) -> Self {
    Self(ResponseBodyInner::FixedSizeBinaryData(data.as_ref().to_vec()))
  }

  pub fn from_file<T: Read + Seek + 'static>(mut file: T) -> io::Result<Self> {
    file.seek(SeekFrom::End(0))?;
    let size = file.stream_position()?;
    Ok(Self(ResponseBodyInner::FixedSizeFile(Box::new(file), size)))
  }

  pub fn from_file_with_chunked_gzip<T: Read + Seek + 'static>(file: T) -> Self {
    Self(ResponseBodyInner::ChunkedGzipFile(Box::new(file)))
  }

  pub fn from_externally_gzipped_file<T: Read + Seek + 'static>(
    mut file_in_gzip_format: T,
  ) -> io::Result<Self> {
    file_in_gzip_format.seek(SeekFrom::End(0))?;
    let size = file_in_gzip_format.stream_position()?;
    Ok(Self(ResponseBodyInner::ExternallyGzippedFile(Box::new(file_in_gzip_format), size)))
  }

  pub fn chunked<T: FnOnce(&dyn ResponseBodySink) -> io::Result<()> + 'static>(
    streamer: T,
  ) -> Self {
    Self(ResponseBodyInner::ChunkedStream(Some(Box::new(streamer))))
  }

  pub fn streamed<T: FnOnce(&dyn ResponseBodySink) -> io::Result<()> + 'static>(
    streamer: T,
  ) -> Self {
    Self(ResponseBodyInner::Stream(Some(Box::new(streamer))))
  }

  /// Creates a response body that streams data from a sink and will on the fly gzip it.
  /// Due to gzip encoding the implementation does not guarantee that each written chunk
  /// corresponds to exactly one http chunk and also does not guarantee that any such chunk is written immediately.
  pub fn chunked_gzip<T: FnOnce(&dyn ResponseBodySink) -> io::Result<()> + 'static>(
    streamer: T,
  ) -> Self {
    Self(ResponseBodyInner::ChunkedGzipStream(Some(Box::new(streamer))))
  }

  pub fn write_to<T: ConnectionStreamWrite + ?Sized>(&mut self, stream: &T) -> io::Result<()> {
    match &mut self.0 {
      ResponseBodyInner::FixedSizeBinaryData(data)
      | ResponseBodyInner::ExternallyGzippedData(data) => stream.write_all(data.as_slice()),
      ResponseBodyInner::FixedSizeTextData(text) => stream.write_all(text.as_bytes()),
      ResponseBodyInner::FixedSizeFile(file, size)
      | ResponseBodyInner::ExternallyGzippedFile(file, size) => {
        //TODO give option via cfg-if to move this to heap. Some unix systems only have 80kb stack and stuff like this has blown up in my face before.
        let mut io_buf = [0u8; 0x1_00_00];
        let mut written = 0u64;
        file.seek(io::SeekFrom::Start(0))?;
        loop {
          let read = file.read(io_buf.as_mut_slice())?;
          if read == 0 {
            if written != *size {
              return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "size of the file changed while writing it to network",
              ));
            }
            return Ok(());
          }

          stream.write_all(
            io_buf
              .get_mut(..read)
              .ok_or(io::Error::new(io::ErrorKind::Other, "buffer overflow"))?,
          )?;
          written = written
            .checked_add(
              u64::try_from(read)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "usize->u64 failed"))?,
            )
            .ok_or(io::Error::new(io::ErrorKind::Other, "u64 overflow"))?;
        }
      }
      ResponseBodyInner::Stream(handler) => handler.take().ok_or_else(|| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "stream can only be written once")
      })?(&StreamSink(stream.as_stream_write())),

      ResponseBodyInner::ChunkedStream(handler) => {
        let sink = ChunkedSink(stream.as_stream_write());
        handler.take().ok_or_else(|| {
          io::Error::new(io::ErrorKind::UnexpectedEof, "stream can only be written once")
        })?(&sink)?;
        sink.finish()
      }
      ResponseBodyInner::ChunkedGzipStream(handler) => {
        let sink = GzipChunkedSink::new(stream.as_stream_write())?;
        handler.take().ok_or_else(|| {
          io::Error::new(io::ErrorKind::UnexpectedEof, "stream can only be written once")
        })?(&sink)?;
        sink.finish()
      }
      ResponseBodyInner::ChunkedGzipFile(file) => {
        file.seek(io::SeekFrom::Start(0))?;
        let sink = GzipChunkedSink::new(stream.as_stream_write())?;
        let mut io_buf = [0u8; 0x1_00_00];
        loop {
          let count = file.read(io_buf.as_mut_slice())?;
          if count == 0 {
            break;
          }
          sink.write_all(&io_buf[..count])?;
        }
        sink.finish()
      }
    }
  }

  pub fn is_chunked(&self) -> bool {
    match self.0 {
      ResponseBodyInner::ChunkedStream(_)
      | ResponseBodyInner::ChunkedGzipStream(_)
      | ResponseBodyInner::ChunkedGzipFile(_) => true,
      _ => false,
    }
  }

  pub fn get_content_encoding(&self) -> Option<&'static str> {
    Some(match self.0 {
      ResponseBodyInner::ExternallyGzippedData(_) => "gzip",
      ResponseBodyInner::ExternallyGzippedFile(_, _) => "gzip",
      ResponseBodyInner::ChunkedGzipStream(_) => "gzip",
      ResponseBodyInner::ChunkedGzipFile(_) => "gzip",
      _ => return None,
    })
  }

  pub fn content_length(&self) -> Option<u64> {
    match &self.0 {
      ResponseBodyInner::FixedSizeBinaryData(data) => u64::try_from(data.len()).ok(),
      ResponseBodyInner::FixedSizeTextData(data) => u64::try_from(data.len()).ok(),
      ResponseBodyInner::FixedSizeFile(_, sz) => Some(*sz),
      ResponseBodyInner::ExternallyGzippedData(data) => u64::try_from(data.len()).ok(),
      ResponseBodyInner::ExternallyGzippedFile(_, sz) => Some(*sz),
      _ => None,
    }
  }
}

pub struct ResponseBodySinkAsWrite<'a>(&'a dyn ResponseBodySink);

impl Write for ResponseBodySinkAsWrite<'_> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.0.write(buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    //Noop
    Ok(())
  }
}

struct StreamSink<'a>(&'a dyn ConnectionStreamWrite);

impl ResponseBodySink for StreamSink<'_> {
  fn write(&self, buffer: &[u8]) -> io::Result<usize> {
    self.0.write(buffer)
  }

  fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
    self.0.write_all(buffer)
  }

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_> {
    ResponseBodySinkAsWrite(self)
  }
}

struct GzipChunkedSink<'a>(RefCell<Option<gzip::Encoder<ChunkedSink<'a>>>>);

impl<'a> GzipChunkedSink<'a> {
  fn new(stream: &'a dyn ConnectionStreamWrite) -> io::Result<GzipChunkedSink<'a>> {
    Ok(Self(RefCell::new(Some(crate::util::new_gzip_encoder(ChunkedSink(stream))?))))
  }
}

impl GzipChunkedSink<'_> {
  fn finish(&self) -> io::Result<()> {
    //Safety, this function will panic/abort if called more than once
    unwrap_some(self.0.borrow_mut().take()).finish().into_result()?.finish()
  }
}

impl ResponseBodySink for GzipChunkedSink<'_> {
  fn write(&self, buffer: &[u8]) -> io::Result<usize> {
    //Safety, this function will panic/abort if called after finish
    unwrap_some(self.0.borrow_mut().as_mut()).write(buffer)
  }

  fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
    //Safety, this function will panic/abort if called after finish
    unwrap_some(self.0.borrow_mut().as_mut()).write_all(buffer)
  }

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_> {
    ResponseBodySinkAsWrite(self)
  }
}

static CHUNK_LUT: [&'static [u8]; 8096] = tii_procmacro::hex_chunked_lut!(8096);

struct ChunkedSink<'a>(&'a dyn ConnectionStreamWrite);

impl Write for ChunkedSink<'_> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    ResponseBodySink::write(self, buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    //NOOP, we only truly flush the response as a whole!
    Ok(())
  }
}

impl ResponseBodySink for ChunkedSink<'_> {
  fn write(&self, buffer: &[u8]) -> io::Result<usize> {
    self.write_all(buffer)?;
    Ok(buffer.len())
  }

  fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
    if buffer.is_empty() {
      return Ok(());
    }

    if let Some(lut) = CHUNK_LUT.get(buffer.len()) {
      self.0.write_all(lut)?;
    } else {
      self.0.write_all(format!("{:X}\r\n", buffer.len()).as_bytes())?;
    }

    self.0.write_all(buffer)?;
    self.0.write_all(b"\r\n")
  }

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_> {
    ResponseBodySinkAsWrite(self)
  }
}

impl ChunkedSink<'_> {
  fn finish(&self) -> io::Result<()> {
    self.0.write_all(b"0\r\n\r\n")
  }
}

impl From<Vec<u8>> for ResponseBody {
  fn from(value: Vec<u8>) -> Self {
    ResponseBody::from_data(value)
  }
}

impl From<String> for ResponseBody {
  fn from(value: String) -> Self {
    ResponseBody::from_data(value.into_bytes())
  }
}

impl From<&str> for ResponseBody {
  fn from(value: &str) -> Self {
    ResponseBody::from_slice(value)
  }
}

impl From<&[u8]> for ResponseBody {
  fn from(value: &[u8]) -> Self {
    ResponseBody::from_slice(value)
  }
}
