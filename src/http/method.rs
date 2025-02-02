//! Provides functionality for handling HTTP methods.

use std::fmt::Display;

/// Represents an HTTP method.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Method {
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

static WELL_KNOWN: &[Method] = &[
  Method::Get,
  Method::Head,
  Method::Post,
  Method::Put,
  Method::Delete,
  Method::Options,
  Method::Trace,
  Method::Patch,
];

impl Method {
  /// Attempts to convert from the HTTP verb into an enum variant.
  ///
  /// ## Example
  /// ```
  /// let method = tii::http::method::Method::from("GET");
  /// assert_eq!(method, tii::http::method::Method::Get);
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
  pub fn well_known() -> &'static [Method] {
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
      Method::Get => "GET",
      Method::Head => "HEAD",
      Method::Post => "POST",
      Method::Put => "PUT",
      Method::Delete => "DELETE",
      Method::Options => "OPTIONS",
      Method::Trace => "TRACE",
      Method::Patch => "PATCH",
      Method::Custom(_) => return None,
    })
  }

  /// returns a &str with the same lifetime as self. this works for custom and none custom methods.
  pub fn as_str(&self) -> &str {
    match self {
      Method::Get => "GET",
      Method::Head => "HEAD",
      Method::Post => "POST",
      Method::Put => "PUT",
      Method::Delete => "DELETE",
      Method::Options => "OPTIONS",
      Method::Trace => "TRACE",
      Method::Patch => "PATCH",
      Method::Custom(meth) => meth.as_str(),
    }
  }
}

impl Display for Method {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}
