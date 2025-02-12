//! Provides functionality for handling HTTP responses.

use crate::http::cookie::SetCookie;
use crate::http::headers::{Headers, HttpHeader, HttpHeaderName};
use crate::http::status::StatusCode;

use crate::http::method::HttpMethod;
use crate::http::mime::MimeType;
use crate::http::request::HttpVersion;
use crate::http::response_body::ResponseBody;
use crate::stream::ConnectionStreamWrite;
use crate::tii_error::{TiiResult, UserError};
use std::io;
use std::io::{Read, Seek};

/// Represents a response from the server.
/// Implements `Into<Vec<u8>>` so can be serialised into bytes to transmit.
///
/// ## Simple Creation
/// ```
/// use tii::MimeType;
/// use tii::StatusCode;
/// tii::Response::ok("Success", MimeType::TextPlain);
/// tii::Response::new(StatusCode::NotFound);
/// ```
///
/// ## Advanced Creation
/// ```
/// tii::Response::new(tii::StatusCode::OK)
///     .with_body_slice(b"Success")
///     .with_header(tii::HttpHeaderName::ContentType, "text/plain");
/// ```
#[derive(Debug)]
pub struct Response {
  /// The status code of the response, for example 200 OK.
  pub status_code: StatusCode,
  /// A list of the headers included in the response.
  pub(crate) headers: Headers,
  /// The body of the response.
  pub body: Option<ResponseBody>,
}

impl Response {
  /// Creates a new response object with the given status code.
  /// Automatically sets the HTTP version to "HTTP/1.1", sets no headers, and creates an empty body.
  pub fn new(status_code: impl Into<StatusCode>) -> Self {
    let status_code = status_code.into();
    Self { status_code, headers: Headers::new(), body: None }
  }

  /// HTTP 200 OK with body.
  pub fn ok(bytes: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Response {
    Self::new(StatusCode::OK)
      .with_body(bytes.into())
      .with_header_unchecked("Content-Type", mime.into().as_str())
  }

  /// HTTP 201 Created with body.
  pub fn created<T: Into<ResponseBody>>(
    bytes: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::Created)
      .with_body(bytes.into())
      .with_header_unchecked("Content-Type", mime.into().as_str())
  }

  /// HTTP 202 Accepted with body.
  pub fn accepted<T: Into<ResponseBody>>(
    bytes: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::Created)
      .with_body(bytes.into())
      .with_header_unchecked("Content-Type", mime.into().as_str())
  }

  /// HTTP 203 Non-Authoritative Information with body
  pub fn non_authoritative(
    bytes: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::NonAuthoritative)
      .with_body(bytes.into())
      .with_header_unchecked("Content-Type", mime.into().as_str())
  }

  /// HTTP 204 No Content
  pub fn no_content() -> Response {
    Self::new(StatusCode::NoContent)
  }

  /// HTTP 205 Reset Content
  pub fn reset_content() -> Response {
    Self::new(StatusCode::ResetContent)
  }

  /// HTTP 206 Partial Content
  /// Note: Content-Range header must still be set by the caller. TODO
  pub fn partial_content(
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::PartialContent)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 300 Multiple Choices
  pub fn multiple_choices(
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::MultipleChoices)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 300 Multiple Choices without body
  pub fn multiple_choices_no_body() -> Response {
    Self::new(StatusCode::MultipleChoices)
  }

  /// HTTP 301 Moved Permanently
  pub fn moved_permanently(
    location: impl AsRef<str>,
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::MovedPermanently)
      .with_header_unchecked(HttpHeaderName::Location, location)
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
      .with_body(body.into())
  }

  /// HTTP 301 Moved Permanently without body
  pub fn moved_permanently_no_body(location: impl AsRef<str>) -> Response {
    Self::new(StatusCode::MovedPermanently)
      .with_header_unchecked(HttpHeaderName::Location, location)
  }

  /// HTTP 302 Found
  pub fn found(
    location: impl AsRef<str>,
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::Found)
      .with_header_unchecked(HttpHeaderName::Location, location)
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
      .with_body(body.into())
  }

  /// HTTP 302 Found without body
  pub fn found_no_body(location: impl AsRef<str>) -> Response {
    Self::new(StatusCode::Found).with_header_unchecked(HttpHeaderName::Location, location)
  }

  /// HTTP 303 See Other
  pub fn see_other(
    location: impl AsRef<str>,
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::SeeOther)
      .with_header_unchecked(HttpHeaderName::Location, location)
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
      .with_body(body.into())
  }

  /// HTTP 303 See Other without body
  pub fn see_other_no_body(location: impl AsRef<str>) -> Response {
    Self::new(StatusCode::SeeOther).with_header_unchecked(HttpHeaderName::Location, location)
  }

  /// HTTP 304 Not modified.
  pub fn not_modified() -> Response {
    Self::new(StatusCode::NotModified)
  }

  /// HTTP 307 Temporary Redirect
  pub fn temporary_redirect(
    location: impl AsRef<str>,
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::TemporaryRedirect)
      .with_header_unchecked(HttpHeaderName::Location, location)
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
      .with_body(body.into())
  }

  /// HTTP 307 Temporary Redirect without body
  pub fn temporary_redirect_no_body(location: impl AsRef<str>) -> Response {
    Self::new(StatusCode::TemporaryRedirect)
      .with_header_unchecked(HttpHeaderName::Location, location)
  }

  /// HTTP 308 Permanent Redirect
  pub fn permanent_redirect(
    location: impl AsRef<str>,
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::PermanentRedirect)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::Location, location)
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 308 Permanent Redirect without body
  pub fn permanent_redirect_no_body(location: impl AsRef<str>) -> Response {
    Self::new(StatusCode::PermanentRedirect)
      .with_header_unchecked(HttpHeaderName::Location, location)
  }

  /// HTTP 400 Bad Request
  pub fn bad_request(
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::BadRequest)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 400 Bad Request without body
  pub fn bad_request_no_body() -> Response {
    Self::new(StatusCode::BadRequest)
  }

  /// HTTP 401 Unauthorized
  pub fn unauthorized() -> Response {
    Self::new(StatusCode::Unauthorized)
  }

  /// HTTP 402 Payment Required with body
  pub fn payment_required(
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::PaymentRequired)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 402 Payment Required without body
  pub fn payment_required_no_body() -> Response {
    Self::new(StatusCode::PaymentRequired)
  }

  /// HTTP 403 Forbidden
  pub fn forbidden(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Response {
    Self::new(StatusCode::Forbidden)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 403 Forbidden
  pub fn forbidden_no_body() -> Response {
    Self::new(StatusCode::Forbidden)
  }

  /// HTTP 404 Not Found with body
  pub fn not_found(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Self {
    Self::new(StatusCode::NotFound)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 404 Not Found without body
  pub fn not_found_no_body() -> Self {
    Self::new(StatusCode::NotFound)
  }

  /// HTTP 405 Method Not Allowed
  pub fn method_not_allowed(allowed_methods: &[HttpMethod]) -> Self {
    if allowed_methods.is_empty() {
      return Self::new(StatusCode::MethodNotAllowed);
    }

    let mut buf = String::new();
    for method in allowed_methods {
      if !buf.is_empty() {
        buf += ", ";
      }
      buf += method.as_str();
    }

    Self::new(StatusCode::MethodNotAllowed)
      .with_header_unchecked(HttpHeaderName::Allow, buf.as_str())
  }

  /// HTTP 406 Not Acceptable
  pub fn not_acceptable(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Self {
    Self::new(StatusCode::NotAcceptable)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 406 Not Acceptable without body
  pub fn not_acceptable_no_body() -> Self {
    Self::new(StatusCode::NotAcceptable)
  }

  /// HTTP 407 Proxy Authentication Required
  pub fn proxy_authentication_required(authenticate: impl AsRef<str>) -> Self {
    Self::new(StatusCode::ProxyAuthenticationRequired)
      .with_header_unchecked(HttpHeaderName::ProxyAuthenticate, authenticate)
  }

  /// HTTP 408 Request Timeout
  pub fn request_timeout() -> Self {
    Self::new(StatusCode::RequestTimeout)
  }

  /// HTTP 409 Conflict
  pub fn conflict(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Self {
    Self::new(StatusCode::Conflict)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 409 Conflict without body
  pub fn conflict_no_body() -> Self {
    Self::new(StatusCode::Conflict)
  }

  /// HTTP 410 Gone
  pub fn gone(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Self {
    Self::new(StatusCode::Gone)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 410 Gone
  pub fn gone_no_body() -> Self {
    Self::new(StatusCode::Gone)
  }

  /// HTTP 411 Length Required
  pub fn length_required(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Self {
    Self::new(StatusCode::LengthRequired)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 411 Length Required without body
  pub fn length_required_no_body() -> Self {
    Self::new(StatusCode::LengthRequired)
  }

  /// HTTP 412 Precondition Failed
  pub fn precondition_failed() -> Self {
    Self::new(StatusCode::PreconditionFailed)
  }

  /// HTTP 413 Content Too Large
  pub fn content_too_large(body: impl Into<ResponseBody>, mime: impl Into<MimeType>) -> Self {
    Self::new(StatusCode::ContentTooLarge)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 413 Content Too Large without body
  pub fn content_too_large_no_body() -> Self {
    Self::new(StatusCode::ContentTooLarge)
  }

  /// HTTP 415 Unsupported Media Type with body
  pub fn unsupported_media_type(
    body: impl Into<ResponseBody>,
    mime: impl Into<MimeType>,
  ) -> Response {
    Self::new(StatusCode::UnsupportedMediaType)
      .with_body(body.into())
      .with_header_unchecked(HttpHeaderName::ContentType, mime.into().as_str())
  }

  /// HTTP 415 Unsupported Media Type without body
  pub fn unsupported_media_type_no_body() -> Response {
    Self::new(StatusCode::UnsupportedMediaType)
  }

  ///Removes the body from the response
  pub fn without_body(mut self) -> Self {
    self.body = None;
    self
  }

  ///Set the body to use with the response
  pub fn with_body(mut self, body: impl Into<ResponseBody>) -> Self {
    self.body = Some(body.into());
    self
  }

  /// Use the string body as request body
  pub fn with_body_string<T: AsRef<str>>(mut self, body: T) -> Self {
    self.body = Some(ResponseBody::from_string(body.as_ref().to_string()));
    self
  }

  /// Use the binary body as request body
  pub fn with_body_vec(mut self, body: Vec<u8>) -> Self {
    self.body = Some(ResponseBody::from_data(body));
    self
  }

  /// Use the binary body as request body
  pub fn with_body_slice<T: AsRef<[u8]>>(mut self, body: T) -> Self {
    self.body = Some(ResponseBody::from_slice(&body));
    self
  }

  /// Use the file (or something file like) as request body
  /// Note: this call fetches the file size which must not change afterward.
  /// This call uses seek to move the file pointer. Any seeking done prior to this call is ignored.
  /// The actual body will always contain the entire "file"
  pub fn with_body_file<T: Read + Seek + 'static>(mut self, body: T) -> io::Result<Self> {
    self.body = Some(ResponseBody::from_file(body)?);
    Ok(self)
  }

  /// Adds the given header to the response.
  /// Returns itself for use in a builder pattern.
  pub fn with_header(mut self, header: impl AsRef<str>, value: impl AsRef<str>) -> TiiResult<Self> {
    self.add_header(header, value)?;
    Ok(self)
  }

  /// Internal add header where the entire state of the request obj is known.
  fn with_header_unchecked(mut self, header: impl AsRef<str>, value: impl AsRef<str>) -> Self {
    self.headers.add(header, value);
    self
  }

  /// Adds the header to the Response.
  pub fn add_header(&mut self, hdr: impl AsRef<str>, value: impl AsRef<str>) -> TiiResult<()> {
    match &hdr.as_ref().into() {
      HttpHeaderName::ContentLength => {
        UserError::ImmutableResponseHeaderModified(HttpHeaderName::ContentLength).into()
      }
      HttpHeaderName::TransferEncoding => {
        UserError::ImmutableResponseHeaderModified(HttpHeaderName::TransferEncoding).into()
      }
      HttpHeaderName::Trailer => {
        UserError::ImmutableResponseHeaderModified(HttpHeaderName::Trailer).into()
      }
      hdr => {
        self.headers.add(hdr, value);
        Ok(())
      }
    }
  }

  /// Replace all header values in the Response
  pub fn set_header(&mut self, header: impl AsRef<str>, value: impl AsRef<str>) -> TiiResult<()> {
    match &header.as_ref().into() {
      HttpHeaderName::ContentLength => {
        UserError::ImmutableResponseHeaderModified(HttpHeaderName::ContentLength).into()
      }
      HttpHeaderName::TransferEncoding => {
        UserError::ImmutableResponseHeaderModified(HttpHeaderName::TransferEncoding).into()
      }
      HttpHeaderName::Trailer => {
        UserError::ImmutableResponseHeaderModified(HttpHeaderName::Trailer).into()
      }
      hdr => {
        self.headers.set(hdr, value);
        Ok(())
      }
    }
  }

  /// remove all values for a given header.
  pub fn remove_header(&mut self, header: impl AsRef<str>) {
    self.headers.remove(header);
  }

  /// Returns an iterator over all headers.
  pub fn get_all_headers(&self) -> impl Iterator<Item = &HttpHeader> {
    self.headers.iter()
  }

  /// Returns the first header or None
  pub fn get_header(&self, name: impl AsRef<str>) -> Option<&str> {
    self.headers.get(name)
  }

  /// Returns the all header values of empty Vec.
  pub fn get_headers(&self, name: impl AsRef<str>) -> Vec<&str> {
    self.headers.get_all(name)
  }

  /// Adds the given cookie to the response in the `Set-Cookie` header.
  /// Returns itself for use in a builder pattern.
  pub fn with_cookie(mut self, cookie: SetCookie) -> Self {
    self.headers.push(cookie.into());
    self
  }

  /// Returns the body as text, if possible.
  pub fn body(&self) -> Option<&ResponseBody> {
    self.body.as_ref()
  }

  ///
  /// Write the request to a streaming output. This consumes the request object.
  ///
  pub fn write_to<T: ConnectionStreamWrite + ?Sized>(
    mut self,
    version: HttpVersion,
    destination: &T,
  ) -> io::Result<()> {
    if version == HttpVersion::Http09 {
      if let Some(body) = self.body.as_mut() {
        body.write_to(destination)?;
      }

      return Ok(());
    }

    destination.write(version.as_net_str().as_bytes())?;
    destination.write(b" ")?;
    destination.write(self.status_code.code_as_utf())?;
    destination.write(b" ")?;
    destination.write(self.status_code.status_line().as_bytes())?;

    for header in self.headers.iter() {
      // TODO should we even have these checks here? they should not be possible.
      if header.name == HttpHeaderName::ContentLength {
        crate::util::unreachable();
      }

      if header.name == HttpHeaderName::TransferEncoding {
        crate::util::unreachable();
      }

      destination.write(b"\r\n")?;
      //TODO remove this clone
      destination.write(header.name.to_string().as_bytes())?;
      destination.write(b": ")?;
      destination.write(header.value.as_bytes())?;
    }

    if let Some(body) = self.body.as_mut() {
      if body.is_chunked() {
        destination.write(b"\r\nTransfer-Encoding: chunked\r\n\r\n")?;
        body.write_to(destination)?;
        destination.flush()?;
        return Ok(());
      }

      if let Some(len) = body.content_length() {
        destination.write(format!("\r\nContent-Length: {}\r\n\r\n", len).as_bytes())?;
      }

      body.write_to(destination)?;
      destination.flush()?;
      return Ok(());
    }

    destination.write(b"\r\nContent-Length: 0\r\n\r\n")?;
    destination.flush()?;
    Ok(())
  }
}
