//! Provides functionality for http response bodies
//! TODO docs before release
#![allow(missing_docs)]

use crate::stream::ConnectionStreamWrite;
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
      ResponseBodyInner::Stream(_) => f.write_str("ResponseBody::Stream(handler)"),
      ResponseBodyInner::ChunkedStream(_) => f.write_str("ResponseBody::ChunkedStream(handler)"),
    }
  }
}

// we don't really exactly need file. We need read+seek.
pub(crate) trait ReadAndSeek: Read + Seek {}

impl<T> ReadAndSeek for T where T: Read + Seek {}

pub trait ResponseBodySink: Write {
  fn write(&self, buffer: &[u8]) -> io::Result<usize>;
  fn write_all(&self, buffer: &[u8]) -> io::Result<()>;

  fn as_write(&self) -> &dyn Write;
}
impl ResponseBody {
  pub fn from_data(data: Vec<u8>) -> Self {
    Self(ResponseBodyInner::FixedSizeBinaryData(data))
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

  pub fn write_to<T: ConnectionStreamWrite + ?Sized>(&mut self, stream: &T) -> io::Result<()> {
    match &mut self.0 {
      ResponseBodyInner::FixedSizeBinaryData(data) => stream.write_all(data.as_slice()),
      ResponseBodyInner::FixedSizeTextData(text) => stream.write_all(text.as_bytes()),
      ResponseBodyInner::FixedSizeFile(file, size) => {
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
    }
  }

  pub fn is_chunked(&self) -> bool {
    matches!(self.0, ResponseBodyInner::ChunkedStream(_))
  }

  pub fn content_length(&self) -> Option<u64> {
    match &self.0 {
      ResponseBodyInner::FixedSizeBinaryData(data) => u64::try_from(data.len()).ok(),
      ResponseBodyInner::FixedSizeTextData(data) => u64::try_from(data.len()).ok(),
      ResponseBodyInner::FixedSizeFile(_, sz) => Some(*sz),
      _ => None,
    }
  }
}

struct StreamSink<'a>(&'a dyn ConnectionStreamWrite);

impl Write for StreamSink<'_> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.0.write(buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    //NOOP, we only truly flush the response as a whole!
    Ok(())
  }
}

impl ResponseBodySink for StreamSink<'_> {
  fn write(&self, buffer: &[u8]) -> io::Result<usize> {
    self.0.write(buffer)
  }

  fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
    self.0.write_all(buffer)
  }

  fn as_write(&self) -> &dyn Write {
    self
  }
}

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

    //TODO implement this with a LUT without a heap allocated String
    let fmt = format!("{:X}\r\n", buffer.len());
    self.0.write_all(fmt.as_bytes())?;
    self.0.write_all(buffer)?;
    self.0.write_all(b"\r\n")
  }

  fn as_write(&self) -> &dyn Write {
    self
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
