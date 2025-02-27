//! Provides functionality for handling HTTP methods.

use std::fmt::Display;

/// Represents an HTTP method.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum HttpMethod {
  /// The `GET` method.
  Get,
  /// The `HEAD` method.
  Head,
  /// The `POST` method.
  Post,
  /// The `PUT` method.
  Put,
  /// The `DELETE` method.
  Delete,
  /// The `OPTIONS` method.
  Options,
  /// The `TRACE` method.
  Trace,
  /// The 'PATCH' method.
  Patch,
  /// Anything else your heart desires.
  Custom(String),
}

impl PartialEq<HttpMethod> for &HttpMethod {
  fn eq(&self, other: &HttpMethod) -> bool {
    self == other
  }
}

static WELL_KNOWN: &[HttpMethod] = &[
  HttpMethod::Get,
  HttpMethod::Head,
  HttpMethod::Post,
  HttpMethod::Put,
  HttpMethod::Delete,
  HttpMethod::Options,
  HttpMethod::Trace,
  HttpMethod::Patch,
];

impl HttpMethod {
  /// Attempts to convert from the HTTP verb into an enum variant.
  ///
  /// ## Example
  /// ```
  /// let method = tii::HttpMethod::from("GET");
  /// assert_eq!(method, tii::HttpMethod::Get);
  /// ```
  pub fn from(name: &str) -> Self {
    match name {
      "GET" => Self::Get,
      "HEAD" => Self::Head,
      "POST" => Self::Post,
      "PUT" => Self::Put,
      "DELETE" => Self::Delete,
      "OPTIONS" => Self::Options,
      "TRACE" => Self::Trace,
      "PATCH" => Self::Patch,
      _ => Self::Custom(name.to_ascii_uppercase()),
    }
  }

  /// Returns an array of all well known http Methods.
  #[must_use]
  pub fn well_known() -> &'static [HttpMethod] {
    WELL_KNOWN
  }

  /// returns true if this is a well known http method.
  pub fn is_well_known(&self) -> bool {
    !matches!(self, Self::Custom(_))
  }

  /// returns true if this is a custom http method.
  pub fn is_custom(&self) -> bool {
    matches!(self, Self::Custom(_))
  }

  /// returns a static &str for well known http methods, returns none for custom http methods.
  #[must_use]
  pub fn well_known_str(&self) -> Option<&'static str> {
    Some(match self {
      HttpMethod::Get => "GET",
      HttpMethod::Head => "HEAD",
      HttpMethod::Post => "POST",
      HttpMethod::Put => "PUT",
      HttpMethod::Delete => "DELETE",
      HttpMethod::Options => "OPTIONS",
      HttpMethod::Trace => "TRACE",
      HttpMethod::Patch => "PATCH",
      HttpMethod::Custom(_) => return None,
    })
  }

  /// returns a &str with the same lifetime as self. this works for custom and none custom methods.
  pub fn as_str(&self) -> &str {
    match self {
      HttpMethod::Get => "GET",
      HttpMethod::Head => "HEAD",
      HttpMethod::Post => "POST",
      HttpMethod::Put => "PUT",
      HttpMethod::Delete => "DELETE",
      HttpMethod::Options => "OPTIONS",
      HttpMethod::Trace => "TRACE",
      HttpMethod::Patch => "PATCH",
      HttpMethod::Custom(meth) => meth.as_str(),
    }
  }

  /// returns true if the server expects that a request that does NOT have a Content-Length: 0 header
  /// with this header may have a body anyway. If this returns true then Tii will be forced to emit
  /// a 'Connection: close' since there may be a body that Tii cannot parse. The body is never parsed in
  /// this instance anyway.
  pub fn is_likely_to_have_request_body(&self) -> bool {
    match self {
      HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Custom(_) => true,
      _ => false,
    }
  }
}

impl Display for HttpMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}
