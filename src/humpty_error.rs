//! Error stuff.
//! TODO docs before release
#![allow(missing_docs)]

use crate::http::headers::HeaderName;
use crate::http::method::Method;
use crate::http::request::HttpVersion;
use crate::http::Response;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
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
pub enum UserCodeError {
  IllegalAcceptHeaderValueSet(String),
  MultipleAcceptHeaderValuesSet(String, String),
  ImmutableRequestHeaderModified(HeaderName, String),
  ImmutableRequestHeaderRemoved(HeaderName),
  ImmutableResponseHeaderModified(HeaderName),
}

impl Display for UserCodeError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO make this not shit
    Debug::fmt(self, f)
  }
}
impl Error for UserCodeError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum HumptyError {
  RequestHeadParsing(RequestHeadParsingError),
  UserCodeError(UserCodeError),
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
      HumptyError::UserCodeError(err) => (err as &mut dyn Error).downcast_mut::<T>(),
      HumptyError::Other(other) => other.downcast_mut::<T>(),
    }
  }

  pub fn downcast_ref<T: Error + Send + 'static>(&self) -> Option<&T> {
    match self {
      HumptyError::IO(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::RequestHeadParsing(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::UserCodeError(err) => (err as &dyn Error).downcast_ref::<T>(),
      HumptyError::Other(other) => other.downcast_ref::<T>(),
    }
  }
  pub fn into_inner(self) -> Box<dyn Error + Send> {
    match self {
      HumptyError::IO(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::RequestHeadParsing(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::UserCodeError(err) => Box::new(err) as Box<dyn Error + Send>,
      HumptyError::Other(other) => other,
    }
  }
}

impl Display for HumptyError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      HumptyError::IO(err) => Display::fmt(err, f),
      HumptyError::RequestHeadParsing(err) => Display::fmt(err, f),
      HumptyError::UserCodeError(err) => Display::fmt(err, f),
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

impl<T> From<UserCodeError> for HumptyResult<T> {
  fn from(value: UserCodeError) -> Self {
    Err(HumptyError::UserCodeError(value))
  }
}
