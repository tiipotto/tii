//! Error stuff.
//! TODO docs before release
#![allow(missing_docs)]

use crate::HttpHeaderName;
use crate::HttpMethod;
use crate::HttpVersion;
use crate::Response;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::io;
use std::io::ErrorKind;

pub type TiiResult<T> = Result<T, TiiError>;

impl From<Response> for TiiResult<Response> {
  fn from(value: Response) -> Self {
    Ok(value)
  }
}

impl From<TiiError> for TiiResult<Response> {
  fn from(value: TiiError) -> Self {
    Err(value)
  }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum RequestHeadParsingError {
  StatusLineContainsInvalidBytes,
  StatusLineNoCRLF,
  StatusLineNoWhitespace,
  StatusLineTooManyWhitespaces,
  StatusLineTooLong(Vec<u8>),
  InvalidPath(String),
  InvalidPathUrlEncoding(String),
  MethodNotSupportedByHttpVersion(HttpVersion, HttpMethod),
  HeaderLineIsNotUsAscii,
  HeaderLineNoCRLF,
  HeaderNameEmpty,
  HeaderValueMissing,
  HeaderValueEmpty,
  HeaderLineTooLong(Vec<u8>),
  HttpVersionNotSupported(String),
  TransferEncodingNotSupported(String),
  ContentEncodingNotSupported(String),
  InvalidContentLength(String),
  ContentLengthHeaderMissing,
  InvalidQueryString(String),
  /// An error occurred during the WebSocket handshake.
  MissingSecWebSocketKeyHeader,
  /// The web socket frame opcode was invalid.
  InvalidWebSocketOpcode,
  UnexpectedWebSocketOpcode,
  WebSocketClosedDuringPendingMessage,
  WebSocketTextMessageIsNotUtf8(Vec<u8>),
}

impl Display for RequestHeadParsingError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO make this not shit
    Debug::fmt(self, f)
  }
}
impl Error for RequestHeadParsingError {}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum UserError {
  IllegalContentTypeHeaderValueSet(String),
  IllegalAcceptHeaderValueSet(String),
  MultipleAcceptHeaderValuesSet(String, String),
  MultipleContentTypeHeaderValuesSet(String, String),
  ImmutableRequestHeaderModified(HttpHeaderName, String),
  ImmutableRequestHeaderRemoved(HttpHeaderName),
  ImmutableResponseHeaderModified(HttpHeaderName),
  RequestHeadBufferTooSmall(usize),
  HeaderNotSupportedByHttpVersion(HttpVersion),
  BadFilterOrBadEndpointCausedEntityTypeMismatch,
}

impl Display for UserError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO make this not shit
    Debug::fmt(self, f)
  }
}
impl Error for UserError {}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum InvalidPathError {
  MorePartsAfterWildcard(String),
  RegexSyntaxError(String, String, String),
  RegexTooBig(String, String, usize),
}
impl Display for InvalidPathError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO make this not shit
    Debug::fmt(self, f)
  }
}
impl Error for InvalidPathError {}

/// Errors that can occur when dynamic types have to be handled
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum TypeSystemError {
  SourceTypeUnknown,
  NoCastToTargetType,
  SourceTypeDoesNotMatch,
  TargetTypeDoesNotMatch,
}

impl Display for TypeSystemError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO
    std::fmt::Debug::fmt(&self, f)
  }
}
impl Error for TypeSystemError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum TiiError {
  RequestHeadParsing(RequestHeadParsingError),
  UserError(UserError),
  InvalidPathError(InvalidPathError),
  IO(io::Error),
  TypeSystem(TypeSystemError),
  Other(Box<dyn Error + Send + Sync>),
}

impl TiiError {
  pub fn new_io<E: Into<Box<dyn Error + Send + Sync>>>(kind: ErrorKind, message: E) -> TiiError {
    io::Error::new(kind, message).into()
  }

  pub fn from_io_kind(kind: ErrorKind) -> TiiError {
    io::Error::from(kind).into()
  }

  pub fn kind(&self) -> ErrorKind {
    match self {
      TiiError::IO(io) => io.kind(),
      TiiError::RequestHeadParsing(_) => ErrorKind::InvalidData,
      _ => ErrorKind::Other,
    }
  }
  pub fn downcast_mut<T: Error + Send + 'static>(&mut self) -> Option<&mut T> {
    match self {
      TiiError::IO(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      TiiError::RequestHeadParsing(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      TiiError::UserError(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      TiiError::InvalidPathError(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      TiiError::TypeSystem(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      TiiError::Other(other) => other.downcast_mut::<T>(),
    }
  }

  pub fn downcast_ref<T: Error + Send + 'static>(&self) -> Option<&T> {
    match self {
      TiiError::IO(err) => (err as &dyn Error).downcast_ref::<T>(),
      TiiError::RequestHeadParsing(err) => (err as &dyn Error).downcast_ref::<T>(),
      TiiError::UserError(err) => (err as &dyn Error).downcast_ref::<T>(),
      TiiError::InvalidPathError(err) => (err as &dyn Error).downcast_ref::<T>(),
      TiiError::TypeSystem(err) => (err as &dyn Error).downcast_ref::<T>(),
      TiiError::Other(other) => other.downcast_ref::<T>(),
    }
  }
  pub fn into_inner(self) -> Box<dyn Error + Send + Sync + 'static> {
    match self {
      TiiError::IO(err) => Box::new(err) as Box<dyn Error + Send + Sync>,
      TiiError::RequestHeadParsing(err) => Box::new(err) as Box<dyn Error + Send + Sync>,
      TiiError::UserError(err) => Box::new(err) as Box<dyn Error + Send + Sync>,
      TiiError::InvalidPathError(err) => Box::new(err) as Box<dyn Error + Send + Sync>,
      TiiError::TypeSystem(err) => Box::new(err) as Box<dyn Error + Send + Sync>,
      TiiError::Other(other) => other,
    }
  }
}

impl Display for TiiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TiiError::IO(err) => Display::fmt(err, f),
      TiiError::RequestHeadParsing(err) => Display::fmt(err, f),
      TiiError::UserError(err) => Display::fmt(err, f),
      TiiError::InvalidPathError(err) => Display::fmt(err, f),
      TiiError::TypeSystem(err) => Display::fmt(err, f),
      TiiError::Other(err) => Display::fmt(err, f),
    }
  }
}

impl<T> From<T> for TiiError
where
  T: Error + Send + Sync + 'static,
{
  fn from(value: T) -> Self {
    let mut dyn_box = Box::new(value) as Box<dyn Error + Send + Sync>;
    dyn_box = match dyn_box.downcast::<io::Error>() {
      Ok(err) => return TiiError::IO(*err),
      Err(err) => err,
    };
    dyn_box = match dyn_box.downcast::<RequestHeadParsingError>() {
      Ok(err) => return TiiError::RequestHeadParsing(*err),
      Err(err) => err,
    };

    TiiError::Other(dyn_box)
  }
}

impl From<TiiError> for Box<dyn Error + Send> {
  fn from(value: TiiError) -> Self {
    value.into_inner()
  }
}

impl<T> From<UserError> for TiiResult<T> {
  fn from(value: UserError) -> Self {
    Err(TiiError::UserError(value))
  }
}

impl From<TiiError> for io::Error {
  fn from(value: TiiError) -> Self {
    match value {
      TiiError::IO(io) => io,
      err => io::Error::new(err.kind(), err.into_inner()),
    }
  }
}
