//! Error stuff.
//! TODO docs before release
#![allow(missing_docs)]

use crate::http::headers::HeaderName;
use crate::http::method::Method;
use crate::http::request::HttpVersion;
use crate::http::Response;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::io;
use std::io::ErrorKind;

pub type HumptyResult<T> = Result<T, HumptyError>;

impl From<Response> for HumptyResult<Response> {
  fn from(value: Response) -> Self {
    Ok(value)
  }
}

impl From<HumptyError> for HumptyResult<Response> {
  fn from(value: HumptyError) -> Self {
    Err(value)
  }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum RequestHeadParsingError {
  EofBeforeReadingAnyBytes,
  StatusLineContainsInvalidBytes,
  StatusLineNoCRLF,
  StatusLineNoWhitespace,
  StatusLineTooManyWhitespaces,
  PathInvalidUrlEncoding(String),
  MethodNotSupportedByHttpVersion(HttpVersion, Method),
  HeaderLineIsNotUsAscii,
  HeaderLineNoCRLF,
  HeaderNameEmpty,
  HeaderValueMissing,
  HeaderValueEmpty,
  HttpVersionNotSupported(String),
  TransferEncodingNotSupported(String),
  InvalidContentLength(String),
  InvalidQueryString(String),
}

/// Represents a WebSocket error.
#[derive(Debug, PartialEq, Eq)]
pub enum WebsocketError {
  /// An error occurred during the WebSocket handshake.
  HandshakeError,
  /// The frame opcode was invalid.
  InvalidOpcode,
  /// The connection has been closed so the request could not be completed.
  ConnectionClosed,
}

impl Display for WebsocketError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl Error for WebsocketError {}


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
  ImmutableRequestHeaderModified(HeaderName, String),
  ImmutableRequestHeaderRemoved(HeaderName),
  ImmutableResponseHeaderModified(HeaderName),
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

#[derive(Debug)]
#[non_exhaustive]
pub enum HumptyError {
  RequestHeadParsing(RequestHeadParsingError),
  UserError(UserError),
  InvalidPathError(InvalidPathError),
  WebsocketError(WebsocketError),
  IO(io::Error),
  Other(Box<dyn Error + Send>),
}

impl HumptyError {
  pub fn new_io<E: Into<Box<dyn Error + Send + Sync>>>(kind: ErrorKind, message: E) -> HumptyError {
    io::Error::new(kind, message).into()
  }

  pub fn kind(&self) -> ErrorKind {
    match self {
      HumptyError::IO(io) => io.kind(),
      HumptyError::RequestHeadParsing(_) => ErrorKind::InvalidData,
      _ => ErrorKind::Other,
    }
  }
  pub fn downcast_mut<T: Error + Send + 'static>(&mut self) -> Option<&mut T> {
    match self {
      HumptyError::IO(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      HumptyError::RequestHeadParsing(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      HumptyError::UserError(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      HumptyError::InvalidPathError(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      HumptyError::WebsocketError(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      HumptyError::Other(other) => other.downcast_mut::<T>(),
    }
  }

  pub fn downcast_ref<T: Error + Send + 'static>(&self) -> Option<&T> {
    match self {
      HumptyError::IO(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::RequestHeadParsing(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::UserError(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::InvalidPathError(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::WebsocketError(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::Other(other) => other.downcast_ref::<T>(),
    }
  }
  pub fn into_inner(self) -> Box<dyn Error + Send> {
    match self {
      HumptyError::IO(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::RequestHeadParsing(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::UserError(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::InvalidPathError(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::WebsocketError(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::Other(other) => other,
    }
  }
}

impl Display for HumptyError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      HumptyError::IO(err) => Display::fmt(err, f),
      HumptyError::RequestHeadParsing(err) => Display::fmt(err, f),
      HumptyError::UserError(err) => Display::fmt(err, f),
      HumptyError::InvalidPathError(err) => Display::fmt(err, f),
      HumptyError::WebsocketError(err) => Display::fmt(err, f),
      HumptyError::Other(err) => Display::fmt(err, f),
    }
  }
}

impl<T> From<T> for HumptyError
where
  T: Error + Send + 'static,
{
  fn from(value: T) -> Self {
    let mut dyn_box = Box::new(value) as Box<dyn Error + Send>;
    dyn_box = match dyn_box.downcast::<io::Error>() {
      Ok(err) => return HumptyError::IO(*err),
      Err(err) => err,
    };
    dyn_box = match dyn_box.downcast::<RequestHeadParsingError>() {
      Ok(err) => return HumptyError::RequestHeadParsing(*err),
      Err(err) => err,
    };

    HumptyError::Other(dyn_box)
  }
}

impl From<HumptyError> for Box<dyn Error + Send> {
  fn from(value: HumptyError) -> Self {
    value.into_inner()
  }
}

impl<T> From<UserError> for HumptyResult<T> {
  fn from(value: UserError) -> Self {
    Err(HumptyError::UserError(value))
  }
}
