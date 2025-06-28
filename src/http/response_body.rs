//! Provides functionality for http response bodies
//! TODO docs before release
#![allow(missing_docs)]

use crate::http::response_entity::ResponseEntity;
use crate::stream::ConnectionStreamWrite;
use crate::util::unwrap_some;
use crate::{
  trace_log, EntitySerializer, MimeType, TiiError, TiiResult, TypeSystem, TypeSystemError,
};
use defer_heavy::defer;
use libflate::gzip;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub(crate) type ResponseBodyHandler = dyn FnOnce(&dyn ResponseBodySink) -> io::Result<()> + Send;

#[repr(transparent)]
#[derive(Debug)]
pub struct ResponseBody(ResponseBodyInner);

// We don't want to expose this enum.
enum ResponseBodyInner {
  Entity(ResponseEntity),

  //Fixed length data, content length header will be set automatically
  FixedSizeBinaryData(Vec<u8>),

  //Fixed length data, content length header will be set automatically
  FixedSizeBinaryDataStaticSlice(&'static [u8]),

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
      ResponseBodyInner::FixedSizeBinaryDataStaticSlice(data) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeBinaryDataStaticSlice({data:?})"))
      }
      ResponseBodyInner::FixedSizeBinaryData(data) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeBinaryData({data:?})"))
      }
      ResponseBodyInner::FixedSizeTextData(data) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeTextData({data:?})"))
      }
      ResponseBodyInner::FixedSizeFile(_, size) => {
        f.write_fmt(format_args!("ResponseBody::FixedSizeFile(file, {size})"))
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
      ResponseBodyInner::Entity(entity) => {
        f.write_fmt(format_args!("ResponseBody::Entity({entity:?})"))
      }
    }
  }
}

// we don't really exactly need file. We need read+seek.
pub(crate) trait ReadAndSeek: Read + Seek + Send {}

impl<T> ReadAndSeek for T where T: Read + Seek + Send {}

pub trait ResponseBodySink {
  fn write(&self, buffer: &[u8]) -> io::Result<usize>;
  fn write_all(&self, buffer: &[u8]) -> io::Result<()>;

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_>;
}
impl ResponseBody {
  pub fn from_entity<T: Any + Send + Debug + 'static>(
    entity: T,
    serializer: impl EntitySerializer<T> + 'static,
  ) -> Self {
    Self(ResponseBodyInner::Entity(ResponseEntity::new(entity, serializer)))
  }
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

  pub fn from_string(data: impl ToString) -> Self {
    Self(ResponseBodyInner::FixedSizeTextData(data.to_string()))
  }

  pub fn from_slice<T: AsRef<[u8]> + ?Sized>(data: &T) -> Self {
    Self(ResponseBodyInner::FixedSizeBinaryData(data.as_ref().to_vec()))
  }

  /// Creates a response body from a static slice.
  /// Unlike the other fn's that accepts borrowed data,
  /// this fn does not copy the slice.
  ///
  /// This is useful for usage with include_bytes!.
  pub fn from_static_slice(data: &'static [u8]) -> Self {
    Self(ResponseBodyInner::FixedSizeBinaryDataStaticSlice(data))
  }

  pub fn from_file<T: Read + Seek + Send + 'static>(mut file: T) -> io::Result<Self> {
    file.seek(SeekFrom::End(0))?;
    let size = file.stream_position()?;
    Ok(Self(ResponseBodyInner::FixedSizeFile(Box::new(file), size)))
  }

  pub fn from_file_with_chunked_gzip<T: Read + Seek + Send + 'static>(file: T) -> Self {
    Self(ResponseBodyInner::ChunkedGzipFile(Box::new(file)))
  }

  pub fn from_externally_gzipped_file<T: Read + Seek + Send + 'static>(
    mut file_in_gzip_format: T,
  ) -> io::Result<Self> {
    file_in_gzip_format.seek(SeekFrom::End(0))?;
    let size = file_in_gzip_format.stream_position()?;
    Ok(Self(ResponseBodyInner::ExternallyGzippedFile(Box::new(file_in_gzip_format), size)))
  }

  pub fn chunked<T: FnOnce(&dyn ResponseBodySink) -> io::Result<()> + Send + 'static>(
    streamer: T,
  ) -> Self {
    Self(ResponseBodyInner::ChunkedStream(Some(Box::new(streamer))))
  }

  pub fn streamed<T: FnOnce(&dyn ResponseBodySink) -> io::Result<()> + Send + 'static>(
    streamer: T,
  ) -> Self {
    Self(ResponseBodyInner::Stream(Some(Box::new(streamer))))
  }

  /// Creates a response body that streams data from a sink and will on the fly gzip it.
  /// Due to gzip encoding the implementation does not guarantee that each written chunk
  /// corresponds to exactly one http chunk and also does not guarantee that any such chunk is written immediately.
  pub fn chunked_gzip<T: FnOnce(&dyn ResponseBodySink) -> io::Result<()> + Send + 'static>(
    streamer: T,
  ) -> Self {
    Self(ResponseBodyInner::ChunkedGzipStream(Some(Box::new(streamer))))
  }

  /// This fn causes entity data to be serialized into a Vec
  /// After this further dynamic operations on the entity are no longer possible.
  /// This call also Drop's the entity.
  /// It has no effect on other Body types.
  pub fn serialize_entity(self, mime: &MimeType) -> TiiResult<ResponseBody> {
    Ok(match self.0 {
      ResponseBodyInner::Entity(entity) => {
        ResponseBody(ResponseBodyInner::FixedSizeBinaryData(entity.serialize(mime)?))
      }
      other => ResponseBody(other),
    })
  }

  /// If this body refers to a dynamic entity then return a dyn Any handle of the dynamic entity.
  pub fn get_entity(&self) -> Option<&dyn Any> {
    match &self.0 {
      ResponseBodyInner::Entity(entity) => Some(entity.get_entity()),
      _ => None,
    }
  }

  /// If this body refers to a dynamic entity then return a mut dyn Any handle of the dynamic entity.
  pub fn get_entity_mut(&mut self) -> Option<&mut dyn Any> {
    match &mut self.0 {
      ResponseBodyInner::Entity(entity) => Some(entity.get_entity_mut()),
      _ => None,
    }
  }

  pub fn get_entity_serializer(&self) -> Option<&dyn Any> {
    match &self.0 {
      ResponseBodyInner::Entity(entity) => Some(entity.get_serializer()),
      _ => None,
    }
  }

  pub fn get_entity_serializer_mut(&mut self) -> Option<&mut dyn Any> {
    match &mut self.0 {
      ResponseBodyInner::Entity(entity) => Some(entity.get_serializer_mut()),
      _ => None,
    }
  }

  /// Will decode the generic entity into a tuple of Entity, Serializer. Both as Box dyn Any.
  /// If the body is not an entity then this will yield Err(Self)
  #[allow(clippy::type_complexity)] //TODO fix this shit later!
  pub fn try_into_entity(self) -> Result<(Box<dyn Any>, Box<dyn Any>), Self> {
    match self.0 {
      ResponseBodyInner::Entity(entity) => Ok(entity.into_inner()),
      _ => Err(self),
    }
  }

  pub(crate) fn entity_cast<DST: Any + ?Sized + 'static, RET: Any + 'static>(
    &self,
    type_system: &TypeSystem,
    receiver: impl FnOnce(&DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    let ety = self.get_entity().ok_or(TypeSystemError::SourceTypeUnknown)?;
    let caster = type_system.type_cast_wrapper(ety.type_id(), TypeId::of::<DST>())?;
    caster.call(ety, receiver)
  }
  pub(crate) fn entity_cast_mut<DST: Any + ?Sized + 'static, RET: Any + 'static>(
    &mut self,
    type_system: &TypeSystem,
    receiver: impl FnOnce(&mut DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    let ety = self.get_entity_mut().ok_or(TypeSystemError::SourceTypeUnknown)?;
    let caster = type_system.type_cast_wrapper_mut(Any::type_id(ety), TypeId::of::<DST>())?;
    caster.call(ety, receiver)
  }

  /// This function writes the raw bytes of the body to a stream.
  /// It is useful if a filter wishes to retrieve the raw data and inspect it.
  /// This raw data does not have any http specific content or transfer encoding applies and contains the raw bytes
  /// just like the other side should interpret them after undoing the encodings.
  ///
  pub fn write_to_raw(self, mime: &MimeType, stream: &mut impl Write) -> TiiResult<()> {
    match self.0 {
      ResponseBodyInner::FixedSizeBinaryDataStaticSlice(data) => stream.write_all(data)?,
      ResponseBodyInner::FixedSizeBinaryData(data) => stream.write_all(data.as_ref())?,
      ResponseBodyInner::FixedSizeTextData(text) => stream.write_all(text.as_ref())?,
      ResponseBodyInner::FixedSizeFile(mut data, _)
      | ResponseBodyInner::ChunkedGzipFile(mut data) => {
        let mut io_buf = [0u8; 0x1_00_00];
        loop {
          let len = data.read(&mut io_buf)?;
          if len == 0 {
            return Ok(());
          }
          stream.write_all(unwrap_some(io_buf.get(..len)))?
        }
      }
      ResponseBodyInner::Stream(mut handler)
      | ResponseBodyInner::ChunkedStream(mut handler)
      | ResponseBodyInner::ChunkedGzipStream(mut handler) => {
        let sink = RawSink(RefCell::new(stream));
        handler.take().ok_or_else(|| {
          io::Error::new(io::ErrorKind::UnexpectedEof, "stream can only be written once")
        })?(&sink)?;
      }
      ResponseBodyInner::ExternallyGzippedData(data) => {
        let mut io_buf = [0u8; 0x1_00_00];
        let mut dec = gzip::Decoder::new(&*data)?;
        loop {
          let len = dec.read(&mut io_buf)?;
          if len == 0 {
            return Ok(());
          }
          stream.write_all(unwrap_some(io_buf.get(..len)))?
        }
      }
      ResponseBodyInner::ExternallyGzippedFile(data, _) => {
        let mut io_buf = [0u8; 0x1_00_00];
        let mut dec = gzip::Decoder::new(data)?;
        loop {
          let len = dec.read(&mut io_buf)?;
          if len == 0 {
            return Ok(());
          }
          stream.write_all(unwrap_some(io_buf.get(..len)))?
        }
      }
      ResponseBodyInner::Entity(entity) => stream.write_all(&entity.serialize(mime)?)?,
    };

    Ok(())
  }

  /// This fn writes the body to a stream in http format.
  /// Its expected that the stream just finished writing the last header.
  /// This fn will handle things like transfer/content encoding (not the header part)
  /// for the body transparently. So a chunked stream will be in the http chunked format for example.
  pub fn write_to_http<T: ConnectionStreamWrite + ?Sized>(
    self,
    request_id: u128,
    stream: &T,
  ) -> TiiResult<()> {
    match self.0 {
      ResponseBodyInner::FixedSizeBinaryDataStaticSlice(data) => stream.write_all(data)?,
      ResponseBodyInner::FixedSizeBinaryData(data)
      | ResponseBodyInner::ExternallyGzippedData(data) => stream.write_all(data.as_slice())?,
      ResponseBodyInner::FixedSizeTextData(text) => stream.write_all(text.as_bytes())?,
      ResponseBodyInner::FixedSizeFile(mut file, size)
      | ResponseBodyInner::ExternallyGzippedFile(mut file, size) => {
        //TODO give option via cfg-if to move this to heap. Some unix systems only have 80kb stack and stuff like this has blown up in my face before.
        let mut io_buf = [0u8; 0x1_00_00];
        let mut written = 0u64;
        file.seek(io::SeekFrom::Start(0))?;
        loop {
          let read = file.read(io_buf.as_mut_slice())?;
          if read == 0 {
            if written != size {
              return Err(TiiError::from_io_kind(io::ErrorKind::InvalidData));
            }
            return Ok(());
          }

          stream.write_all(io_buf.get_mut(..read).ok_or(io::Error::other("buffer overflow"))?)?;
          written = written
            .checked_add(u64::try_from(read).map_err(|_| io::Error::other("usize->u64 failed"))?)
            .ok_or(io::Error::other("u64 overflow"))?;
        }
      }
      ResponseBodyInner::Stream(mut handler) => {
        handler.take().ok_or_else(|| TiiError::from_io_kind(io::ErrorKind::UnexpectedEof))?(
          &StreamSink(stream.as_stream_write()),
        )?
      }

      ResponseBodyInner::ChunkedStream(mut handler) => {
        let sink = ChunkedSink(request_id, stream.as_stream_write());
        handler.take().ok_or_else(|| {
          io::Error::new(io::ErrorKind::UnexpectedEof, "stream can only be written once")
        })?(&sink)?;
        sink.finish()?
      }
      ResponseBodyInner::ChunkedGzipStream(mut handler) => {
        let sink = GzipChunkedSink::new(request_id, stream.as_stream_write())?;
        handler.take().ok_or_else(|| {
          io::Error::new(io::ErrorKind::UnexpectedEof, "stream can only be written once")
        })?(&sink)?;
        sink.finish()?
      }
      ResponseBodyInner::ChunkedGzipFile(mut file) => {
        file.seek(io::SeekFrom::Start(0))?;
        let sink = GzipChunkedSink::new(request_id, stream.as_stream_write())?;
        let mut io_buf = [0u8; 0x1_00_00];
        loop {
          let count = file.read(io_buf.as_mut_slice())?;
          if count == 0 {
            break;
          }
          sink.write_all(unwrap_some(io_buf.get(..count)))?;
        }
        sink.finish()?
      }
      ResponseBodyInner::Entity(entity) => {
        // This should be unreachable under normal circumstances,
        // if we got here anyway we are writing it in Chunked Transfer Encoding.
        let sink = ChunkedSink(request_id, stream.as_stream_write());
        sink.write_all(&entity.serialize(&MimeType::ApplicationOctetStream)?)?;
        sink.finish()?
      }
    };

    Ok(())
  }

  pub fn is_entity(&self) -> bool {
    matches!(self.0, ResponseBodyInner::Entity(_))
  }

  pub fn is_chunked(&self) -> bool {
    matches!(
      self.0,
      ResponseBodyInner::ChunkedStream(_)
        | ResponseBodyInner::ChunkedGzipStream(_)
        | ResponseBodyInner::ChunkedGzipFile(_)
        | ResponseBodyInner::Entity(_)
    )
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
      ResponseBodyInner::FixedSizeBinaryDataStaticSlice(data) => u64::try_from(data.len()).ok(),
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

struct GzipChunkedSink<'a>(u128, RefCell<Option<gzip::Encoder<BufWriter<ChunkedSink<'a>>>>>);

impl<'a> GzipChunkedSink<'a> {
  fn new(
    request_id: u128,
    stream: &'a dyn ConnectionStreamWrite,
  ) -> io::Result<GzipChunkedSink<'a>> {
    // We need BufWriter here because the gzip encoder calls write with like 2-4 bytes at a time.
    // We don't want to emit a http chunk every single time the gzip encoder writes a single symbol
    // the overhead would be several 100%.
    // If we use the BufWriter the overhead only exist when gzip calls flush().
    // This only happens when there is significant data buffered so it's reasonable to emit a chunk then.
    Ok(Self(
      request_id,
      RefCell::new(Some(crate::util::new_gzip_encoder(BufWriter::new(ChunkedSink(
        request_id, stream,
      )))?)),
    ))
  }
}

impl GzipChunkedSink<'_> {
  fn finish(&self) -> io::Result<()> {
    trace_log!("tii: Request {} GzipChunkedSink::finish", self.0);
    defer! {
      trace_log!("tii: Request {} GzipChunkedSink::finish done", self.0);
    }
    //Safety, this function will panic/abort if called more than once
    unwrap_some(self.1.borrow_mut().take()).finish().into_result()?.into_inner()?.finish()
  }
}

impl ResponseBodySink for GzipChunkedSink<'_> {
  fn write(&self, buffer: &[u8]) -> io::Result<usize> {
    trace_log!("tii: Request {} GzipChunkedSink::write with {} bytes", self.0, buffer.len());
    //Safety, this function will panic/abort if called after finish
    unwrap_some(self.1.borrow_mut().as_mut()).write(buffer)
  }

  fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
    trace_log!("tii: Request {} GzipChunkedSink::write_all with {} bytes", self.0, buffer.len());
    //Safety, this function will panic/abort if called after finish
    unwrap_some(self.1.borrow_mut().as_mut()).write_all(buffer)
  }

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_> {
    ResponseBodySinkAsWrite(self)
  }
}

struct RawSink<'a>(RefCell<&'a mut dyn Write>);

impl ResponseBodySink for RawSink<'_> {
  fn write(&self, buffer: &[u8]) -> io::Result<usize> {
    self.0.borrow_mut().write(buffer)
  }

  fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
    self.0.borrow_mut().write_all(buffer)
  }

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_> {
    ResponseBodySinkAsWrite(self)
  }
}
static CHUNK_LUT: [&[u8]; 8096] = tii_procmacro::hex_chunked_lut!(8096);

struct ChunkedSink<'a>(u128, &'a dyn ConnectionStreamWrite);

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

    trace_log!(
      "tii: Request {} ChunkedSink -> Emitting a HTTP chunk with {} bytes",
      self.0,
      buffer.len()
    );

    if let Some(lut) = CHUNK_LUT.get(buffer.len()) {
      self.1.write_all(lut)?;
    } else {
      self.1.write_all(format!("{:X}\r\n", buffer.len()).as_bytes())?;
    }

    self.1.write_all(buffer)?;
    self.1.write_all(b"\r\n")
  }

  fn as_write(&self) -> ResponseBodySinkAsWrite<'_> {
    ResponseBodySinkAsWrite(self)
  }
}

impl ChunkedSink<'_> {
  fn finish(&self) -> io::Result<()> {
    trace_log!("tii: Request {} ChunkedSink -> Emitting trailer", self.0);
    self.1.write_all(b"0\r\n\r\n")
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

impl TryFrom<File> for ResponseBody {
  type Error = io::Error;
  fn try_from(value: File) -> Result<Self, Self::Error> {
    ResponseBody::from_file(value)
  }
}

impl TryFrom<&Path> for ResponseBody {
  type Error = io::Error;
  fn try_from(value: &Path) -> Result<Self, Self::Error> {
    File::open(Path::new(value))?.try_into()
  }
}

impl TryFrom<&PathBuf> for ResponseBody {
  type Error = io::Error;
  fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
    File::open(Path::new(value))?.try_into()
  }
}

impl TryFrom<PathBuf> for ResponseBody {
  type Error = io::Error;
  fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
    File::open(Path::new(value.as_path()))?.try_into()
  }
}
