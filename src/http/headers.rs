//! Provides functionality for handling HTTP headers.

use std::fmt::Display;

/// Represents a collection of headers as part of a request or response.
///
/// Headers can be added with the following methods:
///   - `add(HeaderType::ContentType, "text/html")`: create and add a header
///   - `push(Header::new(HeaderType::ContentType, "text/html"))`: add an existing header
///
/// Anywhere where you would specify the header type, e.g. `HeaderType::ContentType`, you can replace it
///   with the string name of the header, e.g. `Content-Type`, since both these types implement `HeaderLike`.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct Headers(Vec<TiiHttpHeader>);

/// Represents an individual header.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TiiHttpHeader {
  /// The name of the header.
  pub name: TiiHttpHeaderName,
  /// The value of the header.
  pub value: String,
}

//TODO later move this impl to RequestContext and get rid of this unneeded wrapper for the Vec
impl Headers {
  /// Create an empty collection of headers.
  pub fn new() -> Self {
    Self::default()
  }

  /// Get the number of headers in the collection.
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Create and add a new header with the given name and value.
  pub fn add(&mut self, name: impl AsRef<str>, value: impl AsRef<str>) {
    self.0.push(TiiHttpHeader::new(name, value));
  }

  /// Add an existing header to the collection.
  pub fn push(&mut self, header: TiiHttpHeader) {
    self.0.push(header);
  }

  /// Get a reference to the value of the first header with the given name.
  ///
  /// You can either specify the header type as a `HeaderType`, e.g. `HeaderType::ContentType`, or as
  ///   a string, e.g. `Content-Type`.
  pub fn get(&self, name: impl AsRef<str>) -> Option<&str> {
    self.0.iter().find(|h| h.name == name.as_ref().into()).map(|h| h.value.as_str())
  }

  /// Removes all previous instances of the header and sets the header to the single value.
  /// Its guaranteed that the header is only present exactly once after this call returns.
  pub fn set(&mut self, name: impl AsRef<str>, value: impl AsRef<str>) {
    self.remove(&name);
    self.add(name, value);
  }

  /// Will Set the header value if it is not already set.
  /// Should the value already be set then the previous value is returned as Some().
  /// Returns None if the value was set.
  pub fn try_set(&mut self, header: impl AsRef<str>, value: impl AsRef<str>) -> Option<&str> {
    if self.get(&header).is_some() {
      return self.get(header);
    }
    self.0.push(TiiHttpHeader::new(header, value));
    None
  }

  /// Replaces all header values with a single header.
  /// The returned Vec contains the removed values. is len() == 0 if there were none.
  pub fn replace_all(
    &mut self,
    name: impl AsRef<str>,
    value: impl AsRef<str>,
  ) -> Vec<TiiHttpHeader> {
    let mut hcopy = Vec::with_capacity(self.len());
    let mut hrem = Vec::new();
    std::mem::swap(&mut self.0, &mut hcopy);
    for h in hcopy {
      if h.name == name.as_ref().into() {
        hrem.push(h);
        continue;
      }

      self.0.push(h);
    }

    self.0.push(TiiHttpHeader::new(name, value));
    hrem
  }

  /// Get a list of all the values of the headers with the given name.
  /// If no headers with the given name exist, an empty list is returned.
  pub fn get_all(&self, name: impl AsRef<str>) -> Vec<&str> {
    self.0.iter().filter(|h| h.name == name.as_ref().into()).map(|h| h.value.as_str()).collect()
  }

  /// Remove all headers with the given name.
  pub fn remove(&mut self, name: impl AsRef<str>) {
    self.0.retain(|h| h.name != name.as_ref().into());
  }

  /// Return an iterator over the headers in the collection.
  pub fn iter(&self) -> impl Iterator<Item = &TiiHttpHeader> {
    self.0.iter()
  }
}

impl TiiHttpHeader {
  /// Create a new header with the given name and value.
  ///
  /// You can either specify the header type as a `HeaderType`, e.g. `HeaderType::ContentType`, or as
  ///   a string, e.g. `Content-Type`.
  pub fn new(name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
    Self { name: TiiHttpHeaderName::from(name.as_ref()), value: value.as_ref().to_string() }
  }
}

/// Represents a header received in a request.
///TODO implement to &str fn to prevent clone on serialization!

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TiiHttpHeaderName {
  /// Informs the server about the types of data that can be sent back.
  Accept,
  /// Informs the server about the accepted character encodings.
  AcceptCharset,
  /// Indicates the content encoding(s) understood by the client, usually compression algorithms.
  AcceptEncoding,
  /// Informs the server about the client's language(s).
  AcceptLanguage,
  /// Indicates the method that will be used for the actual request when performing an OPTIONS request.
  AccessControlRequestMethod,
  /// Indicates the headers that will be used for the actual request when performing an OPTIONS request.
  AccessControlRequestHeaders,
  /// Provides credentials for HTTP authentication.
  Authorization,
  /// Indicates how the cache should behave.
  CacheControl,
  /// Indicates what should happen to the connection after the request is served.
  Connection,
  /// Lists any encodings used on the payload.
  ContentEncoding,
  /// Indicates the length of the payload body.
  ContentLength,
  /// Indicates the MIME type of the payload body.
  ContentType,
  /// Shares any applicable HTTP cookies with the server.
  Cookie,
  /// Indicates the date and time at which the request was sent.
  Date,
  /// Indicates any expectations that must be met by the server in order to properly serve the request.
  Expect,
  /// May contain reverse proxy information, generally not used in favour of the `X-Forwarded-For` header.
  Forwarded,
  /// Indicates the email address of the client, often used by crawlers.
  From,
  /// Specifies the host to which the request is being sent, e.g. "www.example.com".
  Host,
  /// Indicates the origin that caused the request.
  Origin,
  /// Contains backwards-compatible caching information.
  Pragma,
  /// Indicates the absolute or partial address of the page making the request.
  Referer,
  /// Indicates that the connection is to be upgraded to a different protocol, e.g. WebSocket.
  Upgrade,
  /// Informs the server of basic browser and device information.
  UserAgent,
  /// Contains the addresses of proxies through which the request has been forwarded.
  Via,
  /// Contains information about possible problems with the request.
  Warning,

  /// Indicates whether the response can be shared with other origins.
  AccessControlAllowOrigin,
  /// Indicates whether certain headers can be set.
  AccessControlAllowHeaders,
  /// Indicates whether certain methods can be used.
  AccessControlAllowMethods,
  /// Contains the time in seconds that the object has been cached.
  Age,
  /// The set of methods supported by the resource.
  Allow,
  /// Indicates whether the response is to be displayed as a webpage or downloaded directly.
  ContentDisposition,
  /// Informs the client of the language of the payload body.
  ContentLanguage,
  /// Indicates an alternative location for the returned data.
  ContentLocation,
  /// Identifies a specific version of a resource.
  ETag,
  /// Contains the date and time at which the response is considered expired.
  Expires,
  /// Indicates the date and time at which the response was last modified.
  LastModified,
  /// Provides a means for serialising links in the headers, equivalent to the HTML `<link>` element.
  Link,
  /// Indicates the location at which the resource can be found, used for redirects.
  Location,
  /// Contains information about the server which served the request.
  Server,
  /// Indicates that the client should set the specified cookies.
  SetCookie,
  /// Indicates the encoding used in the transfer of the payload body.
  TransferEncoding,

  /// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/TE>
  TE,
  /// Trailer encoding
  Trailer,

  /// Indicates that a proxy server in the connection needs authentication.
  ProxyAuthenticate,

  /// Custom header with a lowercase name
  Custom(String),
}

/// Contains a list of all well known headers.
static WELL_KNOWN: &[TiiHttpHeaderName] = &[
  TiiHttpHeaderName::Accept,
  TiiHttpHeaderName::Accept,
  TiiHttpHeaderName::AcceptCharset,
  TiiHttpHeaderName::AcceptEncoding,
  TiiHttpHeaderName::AcceptLanguage,
  TiiHttpHeaderName::AccessControlRequestMethod,
  TiiHttpHeaderName::AccessControlRequestHeaders,
  TiiHttpHeaderName::Authorization,
  TiiHttpHeaderName::CacheControl,
  TiiHttpHeaderName::Connection,
  TiiHttpHeaderName::ContentEncoding,
  TiiHttpHeaderName::ContentLength,
  TiiHttpHeaderName::ContentType,
  TiiHttpHeaderName::Cookie,
  TiiHttpHeaderName::Date,
  TiiHttpHeaderName::Expect,
  TiiHttpHeaderName::Forwarded,
  TiiHttpHeaderName::From,
  TiiHttpHeaderName::Host,
  TiiHttpHeaderName::Origin,
  TiiHttpHeaderName::Pragma,
  TiiHttpHeaderName::Referer,
  TiiHttpHeaderName::Upgrade,
  TiiHttpHeaderName::UserAgent,
  TiiHttpHeaderName::Via,
  TiiHttpHeaderName::Warning,
  TiiHttpHeaderName::AccessControlAllowOrigin,
  TiiHttpHeaderName::AccessControlAllowHeaders,
  TiiHttpHeaderName::AccessControlAllowMethods,
  TiiHttpHeaderName::Age,
  TiiHttpHeaderName::Allow,
  TiiHttpHeaderName::ContentDisposition,
  TiiHttpHeaderName::ContentLanguage,
  TiiHttpHeaderName::ContentLocation,
  TiiHttpHeaderName::ETag,
  TiiHttpHeaderName::Expires,
  TiiHttpHeaderName::LastModified,
  TiiHttpHeaderName::Link,
  TiiHttpHeaderName::Location,
  TiiHttpHeaderName::Server,
  TiiHttpHeaderName::SetCookie,
  TiiHttpHeaderName::TransferEncoding,
  TiiHttpHeaderName::Trailer,
  TiiHttpHeaderName::TE,
  TiiHttpHeaderName::ProxyAuthenticate,
];

impl TiiHttpHeaderName {
  /// Returns a static array of all well known header types
  #[must_use]
  pub fn well_known() -> &'static [TiiHttpHeaderName] {
    WELL_KNOWN
  }

  /// Returns true if the header is not well known and the name is heap allocated.
  #[must_use]
  pub fn is_custom(&self) -> bool {
    self.well_known_str().is_none()
  }

  /// Returns true if the header is well known and not heap allocated.
  #[must_use]
  pub fn is_well_known(&self) -> bool {
    self.well_known_str().is_some()
  }

  /// Returns a &str of the header value without copying.
  /// This has the same lifetime as self because
  /// the header string may be a custom header that is heap allocated.
  #[must_use]
  pub fn to_str(&self) -> &str {
    match self {
      TiiHttpHeaderName::Accept => "Accept",
      TiiHttpHeaderName::AcceptCharset => "Accept-Charset",
      TiiHttpHeaderName::AcceptEncoding => "Accept-Encoding",
      TiiHttpHeaderName::AcceptLanguage => "Accept-Language",
      TiiHttpHeaderName::AccessControlRequestMethod => "Access-Control-Request-Method",
      TiiHttpHeaderName::AccessControlRequestHeaders => "Access-Control-Request-Headers",
      TiiHttpHeaderName::Authorization => "Authorization",
      TiiHttpHeaderName::CacheControl => "Cache-Control",
      TiiHttpHeaderName::Connection => "Connection",
      TiiHttpHeaderName::ContentEncoding => "Content-Encoding",
      TiiHttpHeaderName::ContentLength => "Content-Length",
      TiiHttpHeaderName::ContentType => "Content-Type",
      TiiHttpHeaderName::Cookie => "Cookie",
      TiiHttpHeaderName::Date => "Date",
      TiiHttpHeaderName::Expect => "Expect",
      TiiHttpHeaderName::Forwarded => "Forwarded",
      TiiHttpHeaderName::From => "From",
      TiiHttpHeaderName::Host => "Host",
      TiiHttpHeaderName::Origin => "Origin",
      TiiHttpHeaderName::Pragma => "Pragma",
      TiiHttpHeaderName::Referer => "Referer",
      TiiHttpHeaderName::Upgrade => "Upgrade",
      TiiHttpHeaderName::UserAgent => "User-Agent",
      TiiHttpHeaderName::Via => "Via",
      TiiHttpHeaderName::Warning => "Warning",
      TiiHttpHeaderName::AccessControlAllowOrigin => "Access-Control-Allow-Origin",
      TiiHttpHeaderName::AccessControlAllowHeaders => "Access-Control-Allow-Headers",
      TiiHttpHeaderName::AccessControlAllowMethods => "Access-Control-Allow-Methods",
      TiiHttpHeaderName::Age => "Age",
      TiiHttpHeaderName::Allow => "Allow",
      TiiHttpHeaderName::ContentDisposition => "Content-Disposition",
      TiiHttpHeaderName::ContentLanguage => "Content-Language",
      TiiHttpHeaderName::ContentLocation => "Content-Location",
      TiiHttpHeaderName::ETag => "ETag",
      TiiHttpHeaderName::Expires => "Expires",
      TiiHttpHeaderName::LastModified => "Last-Modified",
      TiiHttpHeaderName::Link => "Link",
      TiiHttpHeaderName::Location => "Location",
      TiiHttpHeaderName::Server => "Server",
      TiiHttpHeaderName::SetCookie => "Set-Cookie",
      TiiHttpHeaderName::TransferEncoding => "Transfer-Encoding",
      TiiHttpHeaderName::ProxyAuthenticate => "Proxy-Authenticate",
      TiiHttpHeaderName::TE => "TE",
      TiiHttpHeaderName::Trailer => "Trailer",
      TiiHttpHeaderName::Custom(name) => name.as_str(),
    }
  }

  /// Return Some with a static lifetime if self is not a heap allocated custom header.
  /// If self is a custom header that is heap allocated (and therefore has a non-static lifetime)
  /// It will return none
  #[must_use]
  pub fn well_known_str(&self) -> Option<&'static str> {
    Some(match self {
      TiiHttpHeaderName::Accept => "Accept",
      TiiHttpHeaderName::AcceptCharset => "Accept-Charset",
      TiiHttpHeaderName::AcceptEncoding => "Accept-Encoding",
      TiiHttpHeaderName::AcceptLanguage => "Accept-Language",
      TiiHttpHeaderName::AccessControlRequestMethod => "Access-Control-Request-Method",
      TiiHttpHeaderName::AccessControlRequestHeaders => "Access-Control-Request-Headers",
      TiiHttpHeaderName::Authorization => "Authorization",
      TiiHttpHeaderName::CacheControl => "Cache-Control",
      TiiHttpHeaderName::Connection => "Connection",
      TiiHttpHeaderName::ContentEncoding => "Content-Encoding",
      TiiHttpHeaderName::ContentLength => "Content-Length",
      TiiHttpHeaderName::ContentType => "Content-Type",
      TiiHttpHeaderName::Cookie => "Cookie",
      TiiHttpHeaderName::Date => "Date",
      TiiHttpHeaderName::Expect => "Expect",
      TiiHttpHeaderName::Forwarded => "Forwarded",
      TiiHttpHeaderName::From => "From",
      TiiHttpHeaderName::Host => "Host",
      TiiHttpHeaderName::Origin => "Origin",
      TiiHttpHeaderName::Pragma => "Pragma",
      TiiHttpHeaderName::Referer => "Referer",
      TiiHttpHeaderName::Upgrade => "Upgrade",
      TiiHttpHeaderName::UserAgent => "User-Agent",
      TiiHttpHeaderName::Via => "Via",
      TiiHttpHeaderName::Warning => "Warning",
      TiiHttpHeaderName::AccessControlAllowOrigin => "Access-Control-Allow-Origin",
      TiiHttpHeaderName::AccessControlAllowHeaders => "Access-Control-Allow-Headers",
      TiiHttpHeaderName::AccessControlAllowMethods => "Access-Control-Allow-Methods",
      TiiHttpHeaderName::Age => "Age",
      TiiHttpHeaderName::Allow => "Allow",
      TiiHttpHeaderName::ContentDisposition => "Content-Disposition",
      TiiHttpHeaderName::ContentLanguage => "Content-Language",
      TiiHttpHeaderName::ContentLocation => "Content-Location",
      TiiHttpHeaderName::ETag => "ETag",
      TiiHttpHeaderName::Expires => "Expires",
      TiiHttpHeaderName::LastModified => "Last-Modified",
      TiiHttpHeaderName::Link => "Link",
      TiiHttpHeaderName::Location => "Location",
      TiiHttpHeaderName::Server => "Server",
      TiiHttpHeaderName::SetCookie => "Set-Cookie",
      TiiHttpHeaderName::TransferEncoding => "Transfer-Encoding",
      TiiHttpHeaderName::ProxyAuthenticate => "Proxy-Authenticate",
      TiiHttpHeaderName::Trailer => "Trailer",
      TiiHttpHeaderName::TE => "TE",
      TiiHttpHeaderName::Custom(_) => return None,
    })
  }
}

impl PartialOrd for TiiHttpHeaderName {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TiiHttpHeaderName {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.to_str().cmp(other.to_str())
  }
}

impl From<&str> for TiiHttpHeaderName {
  fn from(name: &str) -> Self {
    //TODO to_ascii_lowercase is a heap allocation...
    match name.to_ascii_lowercase().as_str() {
      "accept" => Self::Accept,
      "accept-charset" => Self::AcceptCharset,
      "accept-encoding" => Self::AcceptEncoding,
      "accept-language" => Self::AcceptLanguage,
      "access-control-request-method" => Self::AccessControlRequestMethod,
      "access-control-request-headers" => Self::AccessControlRequestHeaders,
      "authorization" => Self::Authorization,
      "cache-control" => Self::CacheControl,
      "connection" => Self::Connection,
      "content-encoding" => Self::ContentEncoding,
      "content-length" => Self::ContentLength,
      "content-type" => Self::ContentType,
      "cookie" => Self::Cookie,
      "date" => Self::Date,
      "expect" => Self::Expect,
      "forwarded" => Self::Forwarded,
      "from" => Self::From,
      "host" => Self::Host,
      "origin" => Self::Origin,
      "pragma" => Self::Pragma,
      "referer" => Self::Referer,
      "upgrade" => Self::Upgrade,
      "user-agent" => Self::UserAgent,
      "via" => Self::Via,
      "warning" => Self::Warning,
      "access-control-allow-origin" => Self::AccessControlAllowOrigin,
      "access-control-allow-headers" => Self::AccessControlAllowHeaders,
      "access-control-allow-methods" => Self::AccessControlAllowMethods,
      "age" => Self::Age,
      "allow" => Self::Allow,
      "content-disposition" => Self::ContentDisposition,
      "content-language" => Self::ContentLanguage,
      "content-location" => Self::ContentLocation,
      "etag" => Self::ETag,
      "expires" => Self::Expires,
      "last-modified" => Self::LastModified,
      "link" => Self::Link,
      "location" => Self::Location,
      "server" => Self::Server,
      "set-cookie" => Self::SetCookie,
      "transfer-encoding" => Self::TransferEncoding,
      "proxy-authenticate" => Self::ProxyAuthenticate,
      "te" => Self::TE,
      "trailer" => Self::Trailer,
      _ => Self::Custom(name.to_string()),
    }
  }
}

impl Display for TiiHttpHeaderName {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.to_str())
  }
}

impl AsRef<str> for TiiHttpHeaderName {
  fn as_ref(&self) -> &str {
    self.to_str()
  }
}

#[test]
fn test_header_replace_all() {
  let mut n = Headers::new();
  assert!(n.0.is_empty());
  n.add("Some", "Header");
  n.add("Another", "Value");
  n.add("Another", "Meep");
  n.add("Mop", "Dop");
  let mut it = n.iter();
  assert_eq!(TiiHttpHeader::new("Some", "Header"), it.next().unwrap().clone());
  assert_eq!(TiiHttpHeader::new("Another", "Value"), it.next().unwrap().clone());
  assert_eq!(TiiHttpHeader::new("Another", "Meep"), it.next().unwrap().clone());
  assert_eq!(TiiHttpHeader::new("Mop", "Dop"), it.next().unwrap().clone());
  assert!(it.next().is_none());
  drop(it);

  let rmoved = n.replace_all("Another", "Friend");
  let mut it = n.iter();
  assert_eq!(TiiHttpHeader::new("Some", "Header"), it.next().unwrap().clone());
  assert_eq!(TiiHttpHeader::new("Mop", "Dop"), it.next().unwrap().clone());
  assert_eq!(TiiHttpHeader::new("Another", "Friend"), it.next().unwrap().clone());
  assert!(it.next().is_none());

  let mut it = rmoved.iter();
  assert_eq!(TiiHttpHeader::new("Another", "Value"), it.next().unwrap().clone());
  assert_eq!(TiiHttpHeader::new("Another", "Meep"), it.next().unwrap().clone());
  assert!(it.next().is_none());
}
