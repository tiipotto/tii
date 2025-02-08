//! Provides functionality for handling HTTP status codes.

use crate::util::three_digit_to_utf;

/// Represents an HTTP status code.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TiiStatusCode {
  /// `100 Continue`: Continue with request.
  Continue,
  /// `101 Switching Protocols`: Protocol upgrade.
  SwitchingProtocols,
  /// `200 OK`: Request succeeded.
  OK,
  /// `201 Created`: Resource created.
  Created,
  /// `202 Accepted`: Request received, but not yet acted upon.
  Accepted,
  /// `203 Non-Authoritative Information`: Request processed, but response is from another source.
  NonAuthoritative,
  /// `204 No Content`: There is no content to send for this request.
  NoContent,
  /// `205 Reset Content`: Indicates that the document which sent this request should be reset.
  ResetContent,
  /// `206 Partial Content`: This response only contains part of a resource.
  PartialContent,
  /// `300 Multiple Choice`: The request has multiple possible responses.
  MultipleChoices,
  /// `301 Moved Permanently`: The resource has moved permanently to a new location.
  MovedPermanently,
  /// `302 Found`: The resource has moved temporarily to a new location.
  Found,
  /// `303 See Other`: The resource can be found under a different URI.
  SeeOther,
  /// `304 Not Modified`: The resource has not been modified since the last request.
  NotModified,
  /// `305 Use Proxy`: The requested resource must be accessed through a proxy.
  UseProxy,
  /// `307 Temporary Redirect`: The resource has moved temporarily to a new location.
  TemporaryRedirect,
  /// `308 Permanent Redirect`: The resource has moved permanently to a new location.
  PermanentRedirect,
  /// `400 Bad Request`: The request could not be understood by the server.
  BadRequest,
  /// `401 Unauthorized`: The request requires user authentication.
  Unauthorized,
  /// `402 Payment Required`: Reserved non-standard status code.
  /// Usually used to indicate that an application wants more money to perform an operation.
  PaymentRequired,
  /// `403 Forbidden`: The client is not allowed to access this content.
  Forbidden,
  /// `404 Not Found`: The server can not find the requested resource.
  NotFound,
  /// `405 Method Not Allowed`: The method specified in the request is not allowed for the resource.
  MethodNotAllowed,
  /// `406 Not Acceptable`: No content that meets the criteria is available.
  NotAcceptable,
  /// `407 Proxy Authentication Required`: The client must first authenticate itself with a proxy.
  ProxyAuthenticationRequired,
  /// `408 Request Timeout`: The server timed out waiting for the request.
  RequestTimeout,
  /// `409 Conflict`: The request could not be completed because of a conflict with the server's current state.
  Conflict,
  /// `410 Gone`: The requested resource is no longer available.
  Gone,
  /// `411 Length Required`: The request did not specify the length of its content.
  LengthRequired,
  /// `412 Precondition Failed`: The server does not meet one of the client's preconditions.
  PreconditionFailed,
  /// `413 Payload Too Large`: The request is larger than the server is willing or able to process.
  #[deprecated]
  RequestEntityTooLarge,
  /// `413 Content Too Large`: The request is larger than the server is willing or able to process. Newer version of RequestEntityTooLarge
  ContentTooLarge,
  /// `414 URI Too Long`: The URI provided was too long for the server to process.
  RequestURITooLong,
  /// `415 Unsupported Media Type`: The request entity has a media type which the server or resource does not support.
  UnsupportedMediaType,
  /// `416 Requested Range Not Satisfiable`: The range specified in the `Range` header cannot be fulfilled.
  RequestedRangeNotSatisfiable,
  /// `417 Expectation Failed`: The expectation given in the `Expect` header could not be met by the server.
  ExpectationFailed,
  /// `500 Internal Server Error`: The server encountered an unexpected error which prevented it from fulfilling the request.
  InternalServerError,
  /// `501 Not Implemented`: The server does not support the functionality required to fulfill the request.
  NotImplemented,
  /// `502 Bad Gateway`: The server, while acting as a gateway or proxy, received an invalid response from the upstream server.
  BadGateway,
  /// `503 Service Unavailable`: The server is temporarily unable to handle the request.
  ServiceUnavailable,
  /// `504 Gateway Timeout`: The server, while acting as a gateway or proxy, did not receive a timely response from the upstream server.
  GatewayTimeout,
  /// `505 HTTP Version Not Supported`: The server does not support the HTTP protocol version used in the request.
  VersionNotSupported,

  /// User defined status code, some applications need non-standard custom status codes.
  CustomStr(u16, [u8; 3], &'static str),
  /// User defined status code, some applications need non-standard custom status codes.
  CustomString(u16, [u8; 3], String),
}

impl TiiStatusCode {
  /// Creates a custom Status code from a static message and code.
  /// Codes with more or less than 3 digits or status lines with invalid content will silently turn into
  /// Internal server error. This method is intended to be called from const code so you
  /// can put your custom codes into const variables.
  ///
  pub const fn from_custom(code: u16, status_line: &'static str) -> Self {
    //TODO verify status_line doesnt contain funny characters here?
    if !status_line.is_ascii() || status_line.is_empty() {
      return Self::InternalServerError;
    }

    if code < 100 || code > 999 {
      return Self::InternalServerError;
    }

    Self::CustomStr(code, three_digit_to_utf(code), status_line)
  }

  /// Creates a custom Status code from a dynamic (possibly heap allocated) message and code.
  /// # Returns
  /// None: Codes with more or less than 3 digits or status lines with invalid content
  ///
  pub fn from_custom_string<T: ToString>(code: u16, status_line: &T) -> Option<Self> {
    let status_line = status_line.to_string();

    //TODO verify status_line doesnt contain funny characters here?
    if !status_line.is_ascii() || status_line.is_empty() {
      return None;
    }

    if !(100..=999).contains(&code) {
      return None;
    }

    if let Some(well_known) = Self::from_well_known_code(code) {
      if well_known.status_line() == status_line.as_str() {
        return Some(well_known);
      }
    }

    Some(Self::CustomString(code, three_digit_to_utf(code), status_line))
  }

  /// This fn returns a status code representing a well known code that is specified in the RFC for http.
  /// If the code is not well known then the fn returns the same value it would return for "500 Internal Server Error"
  ///
  pub const fn from_well_known_code_or_500(code: u16) -> Self {
    match code {
      100 => TiiStatusCode::Continue,
      101 => TiiStatusCode::SwitchingProtocols,
      200 => TiiStatusCode::OK,
      201 => TiiStatusCode::Created,
      202 => TiiStatusCode::Accepted,
      203 => TiiStatusCode::NonAuthoritative,
      204 => TiiStatusCode::NoContent,
      205 => TiiStatusCode::ResetContent,
      206 => TiiStatusCode::PartialContent,
      300 => TiiStatusCode::MultipleChoices,
      301 => TiiStatusCode::MovedPermanently,
      302 => TiiStatusCode::Found,
      303 => TiiStatusCode::SeeOther,
      304 => TiiStatusCode::NotModified,
      305 => TiiStatusCode::UseProxy,
      307 => TiiStatusCode::TemporaryRedirect,
      400 => TiiStatusCode::BadRequest,
      401 => TiiStatusCode::Unauthorized,
      403 => TiiStatusCode::Forbidden,
      404 => TiiStatusCode::NotFound,
      405 => TiiStatusCode::MethodNotAllowed,
      406 => TiiStatusCode::NotAcceptable,
      407 => TiiStatusCode::ProxyAuthenticationRequired,
      408 => TiiStatusCode::RequestTimeout,
      409 => TiiStatusCode::Conflict,
      410 => TiiStatusCode::Gone,
      411 => TiiStatusCode::LengthRequired,
      412 => TiiStatusCode::PreconditionFailed,
      413 => TiiStatusCode::ContentTooLarge,
      414 => TiiStatusCode::RequestURITooLong,
      415 => TiiStatusCode::UnsupportedMediaType,
      416 => TiiStatusCode::RequestedRangeNotSatisfiable,
      417 => TiiStatusCode::ExpectationFailed,
      501 => TiiStatusCode::NotImplemented,
      502 => TiiStatusCode::BadGateway,
      503 => TiiStatusCode::ServiceUnavailable,
      504 => TiiStatusCode::GatewayTimeout,
      505 => TiiStatusCode::VersionNotSupported,
      _ => TiiStatusCode::InternalServerError,
    }
  }

  /// This fn returns a status code representing a well known code that is specified in the RFC for http.
  /// # Returns
  /// None: the code was not well known.
  ///
  pub const fn from_well_known_code(code: u16) -> Option<Self> {
    Some(match code {
      100 => TiiStatusCode::Continue,
      101 => TiiStatusCode::SwitchingProtocols,
      200 => TiiStatusCode::OK,
      201 => TiiStatusCode::Created,
      202 => TiiStatusCode::Accepted,
      203 => TiiStatusCode::NonAuthoritative,
      204 => TiiStatusCode::NoContent,
      205 => TiiStatusCode::ResetContent,
      206 => TiiStatusCode::PartialContent,
      300 => TiiStatusCode::MultipleChoices,
      301 => TiiStatusCode::MovedPermanently,
      302 => TiiStatusCode::Found,
      303 => TiiStatusCode::SeeOther,
      304 => TiiStatusCode::NotModified,
      305 => TiiStatusCode::UseProxy,
      307 => TiiStatusCode::TemporaryRedirect,
      400 => TiiStatusCode::BadRequest,
      401 => TiiStatusCode::Unauthorized,
      403 => TiiStatusCode::Forbidden,
      404 => TiiStatusCode::NotFound,
      405 => TiiStatusCode::MethodNotAllowed,
      406 => TiiStatusCode::NotAcceptable,
      407 => TiiStatusCode::ProxyAuthenticationRequired,
      408 => TiiStatusCode::RequestTimeout,
      409 => TiiStatusCode::Conflict,
      410 => TiiStatusCode::Gone,
      411 => TiiStatusCode::LengthRequired,
      412 => TiiStatusCode::PreconditionFailed,
      413 => TiiStatusCode::ContentTooLarge,
      414 => TiiStatusCode::RequestURITooLong,
      415 => TiiStatusCode::UnsupportedMediaType,
      416 => TiiStatusCode::RequestedRangeNotSatisfiable,
      417 => TiiStatusCode::ExpectationFailed,
      500 => TiiStatusCode::InternalServerError,
      501 => TiiStatusCode::NotImplemented,
      502 => TiiStatusCode::BadGateway,
      503 => TiiStatusCode::ServiceUnavailable,
      504 => TiiStatusCode::GatewayTimeout,
      505 => TiiStatusCode::VersionNotSupported,
      _ => return None,
    })
  }

  /// Returns the status line as an Option<&'static str>
  /// This fn will return None for heap allocated status lines.
  pub const fn status_line_static(&self) -> Option<&'static str> {
    Some(match self {
      TiiStatusCode::Continue => "Continue",
      TiiStatusCode::SwitchingProtocols => "Switching Protocols",
      TiiStatusCode::OK => "OK",
      TiiStatusCode::Created => "Created",
      TiiStatusCode::Accepted => "Accepted",
      TiiStatusCode::NonAuthoritative => "Non-Authoritative Information",
      TiiStatusCode::NoContent => "No Content",
      TiiStatusCode::ResetContent => "Reset Content",
      TiiStatusCode::PartialContent => "Partial Content",
      TiiStatusCode::MultipleChoices => "Multiple Choices",
      TiiStatusCode::MovedPermanently => "Moved Permanently",
      TiiStatusCode::Found => "Found",
      TiiStatusCode::SeeOther => "See Other",
      TiiStatusCode::NotModified => "Not Modified",
      TiiStatusCode::UseProxy => "Use Proxy",
      TiiStatusCode::TemporaryRedirect => "Temporary Redirect",
      TiiStatusCode::BadRequest => "Bad Request",
      TiiStatusCode::Unauthorized => "Unauthorized",
      TiiStatusCode::Forbidden => "Forbidden",
      TiiStatusCode::NotFound => "Not Found",
      TiiStatusCode::MethodNotAllowed => "Method Not Allowed",
      TiiStatusCode::NotAcceptable => "Not Acceptable",
      TiiStatusCode::ProxyAuthenticationRequired => "Proxy Authentication Required",
      TiiStatusCode::RequestTimeout => "Request Timeout",
      TiiStatusCode::Conflict => "Conflict",
      TiiStatusCode::Gone => "Gone",
      TiiStatusCode::LengthRequired => "Length Required",
      TiiStatusCode::PreconditionFailed => "Precondition Failed",
      #[expect(deprecated)]
      TiiStatusCode::RequestEntityTooLarge => "Request Entity Too Large",
      TiiStatusCode::ContentTooLarge => "Content Too Large",
      TiiStatusCode::RequestURITooLong => "Request-URI Too Long",
      TiiStatusCode::UnsupportedMediaType => "Unsupported Media Type",
      TiiStatusCode::RequestedRangeNotSatisfiable => "Requested Range Not Satisfiable",
      TiiStatusCode::ExpectationFailed => "Expectation Failed",
      TiiStatusCode::InternalServerError => "Internal Server Error",
      TiiStatusCode::NotImplemented => "Not Implemented",
      TiiStatusCode::BadGateway => "Bad Gateway",
      TiiStatusCode::ServiceUnavailable => "Service Unavailable",
      TiiStatusCode::GatewayTimeout => "Gateway Timeout",
      TiiStatusCode::VersionNotSupported => "HTTP Version Not Supported",
      TiiStatusCode::PermanentRedirect => "Permanent Redirect",
      TiiStatusCode::PaymentRequired => "Payment Required",
      TiiStatusCode::CustomStr(_, _, str) => str,
      TiiStatusCode::CustomString(_, _, _) => return None,
    })
  }

  /// Returns the status line as a &str
  pub fn status_line(&self) -> &str {
    match self {
      TiiStatusCode::Continue => "Continue",
      TiiStatusCode::SwitchingProtocols => "Switching Protocols",
      TiiStatusCode::OK => "OK",
      TiiStatusCode::Created => "Created",
      TiiStatusCode::Accepted => "Accepted",
      TiiStatusCode::NonAuthoritative => "Non-Authoritative Information",
      TiiStatusCode::NoContent => "No Content",
      TiiStatusCode::ResetContent => "Reset Content",
      TiiStatusCode::PartialContent => "Partial Content",
      TiiStatusCode::MultipleChoices => "Multiple Choices",
      TiiStatusCode::MovedPermanently => "Moved Permanently",
      TiiStatusCode::Found => "Found",
      TiiStatusCode::SeeOther => "See Other",
      TiiStatusCode::NotModified => "Not Modified",
      TiiStatusCode::UseProxy => "Use Proxy",
      TiiStatusCode::TemporaryRedirect => "Temporary Redirect",
      TiiStatusCode::BadRequest => "Bad Request",
      TiiStatusCode::Unauthorized => "Unauthorized",
      TiiStatusCode::Forbidden => "Forbidden",
      TiiStatusCode::NotFound => "Not Found",
      TiiStatusCode::MethodNotAllowed => "Method Not Allowed",
      TiiStatusCode::NotAcceptable => "Not Acceptable",
      TiiStatusCode::ProxyAuthenticationRequired => "Proxy Authentication Required",
      TiiStatusCode::RequestTimeout => "Request Timeout",
      TiiStatusCode::Conflict => "Conflict",
      TiiStatusCode::Gone => "Gone",
      TiiStatusCode::LengthRequired => "Length Required",
      TiiStatusCode::PreconditionFailed => "Precondition Failed",
      #[expect(deprecated)]
      TiiStatusCode::RequestEntityTooLarge => "Request Entity Too Large",
      TiiStatusCode::ContentTooLarge => "Content Too Large",
      TiiStatusCode::RequestURITooLong => "Request-URI Too Long",
      TiiStatusCode::UnsupportedMediaType => "Unsupported Media Type",
      TiiStatusCode::RequestedRangeNotSatisfiable => "Requested Range Not Satisfiable",
      TiiStatusCode::ExpectationFailed => "Expectation Failed",
      TiiStatusCode::InternalServerError => "Internal Server Error",
      TiiStatusCode::NotImplemented => "Not Implemented",
      TiiStatusCode::BadGateway => "Bad Gateway",
      TiiStatusCode::ServiceUnavailable => "Service Unavailable",
      TiiStatusCode::GatewayTimeout => "Gateway Timeout",
      TiiStatusCode::VersionNotSupported => "HTTP Version Not Supported",
      TiiStatusCode::PermanentRedirect => "Permanent Redirect",
      TiiStatusCode::PaymentRequired => "Payment Required",
      TiiStatusCode::CustomStr(_, _, str) => str,
      TiiStatusCode::CustomString(_, _, str) => str.as_str(),
    }
  }

  /// Returns the 3-digit code as 3 byte utf-8 representation.
  pub const fn code_as_utf(&self) -> &[u8; 3] {
    match self {
      TiiStatusCode::Continue => b"100",
      TiiStatusCode::SwitchingProtocols => b"101",
      TiiStatusCode::OK => b"200",
      TiiStatusCode::Created => b"201",
      TiiStatusCode::Accepted => b"202",
      TiiStatusCode::NonAuthoritative => b"203",
      TiiStatusCode::NoContent => b"204",
      TiiStatusCode::ResetContent => b"205",
      TiiStatusCode::PartialContent => b"206",
      TiiStatusCode::MultipleChoices => b"300",
      TiiStatusCode::MovedPermanently => b"301",
      TiiStatusCode::Found => b"302",
      TiiStatusCode::SeeOther => b"303",
      TiiStatusCode::NotModified => b"304",
      TiiStatusCode::UseProxy => b"305",
      TiiStatusCode::TemporaryRedirect => b"307",
      TiiStatusCode::PermanentRedirect => b"308",
      TiiStatusCode::BadRequest => b"400",
      TiiStatusCode::PaymentRequired => b"402",
      TiiStatusCode::Unauthorized => b"401",
      TiiStatusCode::Forbidden => b"403",
      TiiStatusCode::NotFound => b"404",
      TiiStatusCode::MethodNotAllowed => b"405",
      TiiStatusCode::NotAcceptable => b"406",
      TiiStatusCode::ProxyAuthenticationRequired => b"407",
      TiiStatusCode::RequestTimeout => b"408",
      TiiStatusCode::Conflict => b"409",
      TiiStatusCode::Gone => b"410",
      TiiStatusCode::LengthRequired => b"411",
      TiiStatusCode::PreconditionFailed => b"412",
      #[expect(deprecated)]
      TiiStatusCode::RequestEntityTooLarge => b"413",
      TiiStatusCode::ContentTooLarge => b"413",
      TiiStatusCode::RequestURITooLong => b"414",
      TiiStatusCode::UnsupportedMediaType => b"415",
      TiiStatusCode::RequestedRangeNotSatisfiable => b"416",
      TiiStatusCode::ExpectationFailed => b"417",
      TiiStatusCode::InternalServerError => b"500",
      TiiStatusCode::NotImplemented => b"501",
      TiiStatusCode::BadGateway => b"502",
      TiiStatusCode::ServiceUnavailable => b"503",
      TiiStatusCode::GatewayTimeout => b"504",
      TiiStatusCode::VersionNotSupported => b"505",
      TiiStatusCode::CustomStr(_, code, _) => code,
      TiiStatusCode::CustomString(_, code, _) => code,
    }
  }

  /// Returns the code as u16. This value is guaranteed to be in >= 100 <= 999 range.
  pub const fn code(&self) -> u16 {
    match self {
      TiiStatusCode::Continue => 100,
      TiiStatusCode::SwitchingProtocols => 101,
      TiiStatusCode::OK => 200,
      TiiStatusCode::Created => 201,
      TiiStatusCode::Accepted => 202,
      TiiStatusCode::NonAuthoritative => 203,
      TiiStatusCode::NoContent => 204,
      TiiStatusCode::ResetContent => 205,
      TiiStatusCode::PartialContent => 206,
      TiiStatusCode::MultipleChoices => 300,
      TiiStatusCode::MovedPermanently => 301,
      TiiStatusCode::Found => 302,
      TiiStatusCode::SeeOther => 303,
      TiiStatusCode::NotModified => 304,
      TiiStatusCode::UseProxy => 305,
      TiiStatusCode::TemporaryRedirect => 307,
      TiiStatusCode::PermanentRedirect => 308,
      TiiStatusCode::BadRequest => 400,
      TiiStatusCode::Unauthorized => 401,
      TiiStatusCode::PaymentRequired => 402,
      TiiStatusCode::Forbidden => 403,
      TiiStatusCode::NotFound => 404,
      TiiStatusCode::MethodNotAllowed => 405,
      TiiStatusCode::NotAcceptable => 406,
      TiiStatusCode::ProxyAuthenticationRequired => 407,
      TiiStatusCode::RequestTimeout => 408,
      TiiStatusCode::Conflict => 409,
      TiiStatusCode::Gone => 410,
      TiiStatusCode::LengthRequired => 411,
      TiiStatusCode::PreconditionFailed => 412,
      #[expect(deprecated)]
      TiiStatusCode::RequestEntityTooLarge => 413,
      TiiStatusCode::ContentTooLarge => 413,
      TiiStatusCode::RequestURITooLong => 414,
      TiiStatusCode::UnsupportedMediaType => 415,
      TiiStatusCode::RequestedRangeNotSatisfiable => 416,
      TiiStatusCode::ExpectationFailed => 417,
      TiiStatusCode::InternalServerError => 500,
      TiiStatusCode::NotImplemented => 501,
      TiiStatusCode::BadGateway => 502,
      TiiStatusCode::ServiceUnavailable => 503,
      TiiStatusCode::GatewayTimeout => 504,
      TiiStatusCode::VersionNotSupported => 505,
      TiiStatusCode::CustomStr(code, _, _) => *code,
      TiiStatusCode::CustomString(code, _, _) => *code,
    }
  }
}
