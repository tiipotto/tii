//! Provides functionality for handling HTTP requests.

use crate::HttpMethod;
use crate::{trace_log, Cookie};
use crate::{Headers, HttpHeader, HttpHeaderName};

use crate::tii_error::{RequestHeadParsingError, TiiError, TiiResult, UserError};
use crate::util::{unwrap_ok, unwrap_some};
use crate::warn_log;
use crate::ConnectionStream;
use crate::{AcceptQualityMimeType, MimeType, QValue};
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;

/// Enum for http versions tii supports.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[non_exhaustive] //Not sure but I don't want to close the door on http 2 shut!
pub enum HttpVersion {
  /// Earliest http version. Has no concept of request bodies or headers. to trigger a request run `echo -ne 'GET /path/goes/here\r\n' | nc 127.0.0.1 8080`
  /// Responses are just the body, no headers, no nothing.
  Http09,
  /// First actually usable http version. Has headers, bodies, etc. but notably 1 connection per request and thus no transfer encoding
  Http10,
  /// Most recent 1.X version, has all features.
  Http11,
}

impl HttpVersion {
  /// returns the printable name of the http version.
  /// This is not always equivalent to how its appears in binary on the status line.
  pub fn as_str(&self) -> &'static str {
    match self {
      HttpVersion::Http09 => "HTTP/0.9",
      HttpVersion::Http10 => "HTTP/1.0",
      HttpVersion::Http11 => "HTTP/1.1",
    }
  }
  /// returns the network bytes in the status line for the http version.
  pub fn as_net_str(&self) -> &'static str {
    match self {
      HttpVersion::Http09 => "",
      HttpVersion::Http10 => "HTTP/1.0",
      HttpVersion::Http11 => "HTTP/1.1",
    }
  }
}

impl Display for HttpVersion {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      HttpVersion::Http09 => f.write_str("HTTP/0.9"),
      HttpVersion::Http10 => f.write_str("HTTP/1.0"),
      HttpVersion::Http11 => f.write_str("HTTP/1.1"),
    }
  }
}

impl HttpVersion {
  /// Tries to parse the http version part of the status line to a http version.
  /// empty string is treated as http09 because http09 doesn't have a version in its status line.
  /// Returns input on error.
  pub fn try_from_net_str<T: AsRef<str>>(value: T) -> Result<Self, T> {
    match value.as_ref() {
      "HTTP/1.0" => Ok(HttpVersion::Http10),
      "HTTP/1.1" => Ok(HttpVersion::Http11),
      "" => Ok(HttpVersion::Http09),
      _ => Err(value),
    }
  }

  /// Tries to parse the http version from the printable name. This was most likely returned by a call to `HttpVersion::as_str`
  pub fn try_from_str<T: AsRef<str>>(value: T) -> Result<Self, T> {
    match value.as_ref() {
      "HTTP/1.0" => Ok(HttpVersion::Http10),
      "HTTP/1.1" => Ok(HttpVersion::Http11),
      "HTTP/0.9" => Ok(HttpVersion::Http09),
      _ => Err(value),
    }
  }
}

/// Represents a request to the server.
/// Contains parsed information about the request's data.
#[derive(Clone, Debug)]
pub struct RequestHead {
  /// The method used in making the request, e.g. "GET".
  method: HttpMethod,

  /// The HTTP version of the request.
  version: HttpVersion,

  /// The status line as is.
  /// For example "GET /index.html HTTP1.1"
  /// the crlf has been stripped already!
  status_line: String,

  /// The path to which the request was made.
  path: String,

  /// Vec of query parameters, key=value in order of appearance.
  query: Vec<(String, String)>,

  accept: Vec<AcceptQualityMimeType>,

  content_type: Option<MimeType>,

  /// A list of headers included in the request.
  headers: Headers,
}

fn validate_raw_path(raw_path: &str) -> TiiResult<()> {
  //https://datatracker.ietf.org/doc/html/rfc3986#section-3.3
  for n in raw_path.bytes() {
    match n {
      b'/' => {}
      b'-' => {}
      b'.' => {}
      b'_' => {}
      b'~' => {}
      b'!' => {}
      b'$' => {}
      b'\'' => {}
      b'(' => {}
      b')' => {}
      b'*' => {}
      b'+' => {}
      b',' => {}
      b';' => {}
      b'=' => {}
      b':' => {}
      b'@' => {}
      b'%' => {}
      //Curl doesn't escape this and it probably won't cause harm?
      b'\\' => {}
      _ => {
        if !n.is_ascii_alphanumeric() {
          trace_log!("validate_raw_path Err {raw_path} due to byte {n}");
          return Err(TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidPath(
            raw_path.to_string(),
          )));
        }
      }
    }
  }

  Ok(())
}

fn parse_status_line(start_line_buf: &Vec<u8>) -> TiiResult<&str> {
  for n in start_line_buf {
    // https://en.wikipedia.org/wiki/Percent-encoding#Types_of_URI_characters
    // plus space char which we check later...
    match *n {
      //RFC 3986 section 2.2 Reserved Characters
      // TODO some of these chars are not valid for the status line... the status line is not the URI!
      b'!' => {}
      b'$' => {}
      b'&' => {}
      b'\'' => {}
      b'(' => {}
      b')' => {}
      b'*' => {}
      b'+' => {}
      b',' => {}
      b'/' => {}
      b':' => {}
      b';' => {}
      b'=' => {}
      b'?' => {}
      b'@' => {}
      b'[' => {}
      b']' => {}
      // RFC 3986 section 2.3 Unreserved Characters
      b'-' => {}
      b'.' => {}
      b'_' => {}
      b'~' => {}
      //Other Stuff
      b'%' => {}
      b' ' => {}
      b'\\' => {} // curl doesnt escape this character
      b'\r' => {} // TODO we should check this later... this is only allowed as the second to last char...
      b'\n' => {} // TODO we should check this later... this is only allowed as the last char...
      other => {
        if other.is_ascii_alphanumeric() {
          continue;
        }
        return Err(RequestHeadParsingError::StatusLineContainsInvalidBytes.into());
      }
    }
  }

  // We could use the unsafe variant here without issue to prevent double validation our validation is stricter than str validation.
  Ok(
    std::str::from_utf8(start_line_buf)
      .map_err(|_| RequestHeadParsingError::StatusLineContainsInvalidBytes)?,
  )
}

fn parse_raw_query(raw_query: &str) -> TiiResult<Vec<(String, String)>> {
  if raw_query.is_empty() {
    return Ok(Vec::new());
  }

  let mut query = Vec::new();
  let mut current_key = Vec::new();
  let mut current_value = Vec::new();
  let mut matching_value = false;
  for n in raw_query.as_bytes() {
    match *n {
      b'=' => {
        if matching_value {
          return Err(RequestHeadParsingError::InvalidQueryString(raw_query.to_string()).into());
        }
        matching_value = true;
      }
      b'&' => {
        if !matching_value {
          return Err(RequestHeadParsingError::InvalidQueryString(raw_query.to_string()).into());
        }

        let key = urlencoding::decode(unwrap_ok(std::str::from_utf8(current_key.as_slice())))
          .map_err(|_| RequestHeadParsingError::InvalidQueryString(raw_query.to_string()))?
          .to_string();

        let value = urlencoding::decode(unwrap_ok(std::str::from_utf8(current_value.as_slice())))
          .map_err(|_| RequestHeadParsingError::InvalidQueryString(raw_query.to_string()))?
          .to_string();

        query.push((key, value));

        matching_value = false;
        current_key = Vec::new();
        current_value = Vec::new()
      }
      b'!' | b'$' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b'-' | b'.' | b'/' | b':' | b';'
      | b'@' | b'_' | b'~' => {
        if matching_value {
          current_value.push(*n);
        } else {
          current_key.push(*n);
        }
      }
      other => {
        if !other.is_ascii_alphanumeric() {
          return Err(RequestHeadParsingError::InvalidQueryString(raw_query.to_string()).into());
        }

        if matching_value {
          current_value.push(*n);
        } else {
          current_key.push(*n);
        }
      }
    }
  }

  if !matching_value {
    return Err(RequestHeadParsingError::InvalidQueryString(raw_query.to_string()).into());
  }

  let key = urlencoding::decode(unwrap_ok(std::str::from_utf8(current_key.as_slice())))
    .map_err(|_| RequestHeadParsingError::InvalidQueryString(raw_query.to_string()))?
    .to_string();

  let value = urlencoding::decode(unwrap_ok(std::str::from_utf8(current_value.as_slice())))
    .map_err(|_| RequestHeadParsingError::InvalidQueryString(raw_query.to_string()))?
    .to_string();

  query.push((key, value));

  Ok(query)
}

impl RequestHead {
  /// Attempts to read and parse one HTTP request from the given reader.
  pub fn new(stream: &dyn ConnectionStream, max_head_buffer_size: usize) -> TiiResult<Self> {
    let mut start_line_buf: Vec<u8> = Vec::with_capacity(256);
    let count = stream.read_until(0xA, max_head_buffer_size, &mut start_line_buf)?;

    if count == 0 {
      //Unreachable unless stream implementation is shit. TC 42 tests this case.
      return Err(TiiError::from_io_kind(ErrorKind::UnexpectedEof));
    }

    if count == max_head_buffer_size {
      return Err(RequestHeadParsingError::StatusLineTooLong(start_line_buf).into());
    }

    let start_line_string = parse_status_line(&start_line_buf)?;

    let status_line =
      start_line_string.strip_suffix("\r\n").ok_or(RequestHeadParsingError::StatusLineNoCRLF)?;

    let mut start_line = status_line.split(' ');

    let method = HttpMethod::from(unwrap_some(start_line.next()));

    let mut uri_iter =
      start_line.next().ok_or(RequestHeadParsingError::StatusLineNoWhitespace)?.splitn(2, '?');

    let version = start_line
      .next()
      .map(HttpVersion::try_from_net_str)
      .unwrap_or(Ok(HttpVersion::Http09)) //Http 0.9 has no suffix
      .map_err(|version| RequestHeadParsingError::HttpVersionNotSupported(version.to_string()))?;

    if start_line.next().is_some() {
      return Err(TiiError::from(RequestHeadParsingError::StatusLineTooManyWhitespaces));
    }

    let raw_path = unwrap_some(uri_iter.next());
    validate_raw_path(raw_path)?;

    let path = urlencoding::decode(raw_path)
      .map_err(|_| {
        TiiError::from(RequestHeadParsingError::InvalidPathUrlEncoding(raw_path.to_string()))
      })?
      .to_string();

    let raw_query = uri_iter.next().unwrap_or("");
    let query = parse_raw_query(raw_query)?;

    let mut headers = Headers::new();

    if version == HttpVersion::Http09 {
      if method != HttpMethod::Get {
        return Err(TiiError::from(RequestHeadParsingError::MethodNotSupportedByHttpVersion(
          version, method,
        )));
      }

      return Ok(Self {
        method,
        path,
        query,
        version,
        headers,
        content_type: None,
        accept: vec![AcceptQualityMimeType::from_mime(MimeType::TextHtml, QValue::default())], // Http 0.9 only accepts html.
        status_line: status_line.to_string(),
      });
    }

    loop {
      let mut line_buf: Vec<u8> = Vec::with_capacity(256);
      let count = stream.read_until(0xA, max_head_buffer_size, &mut line_buf)?;

      if count == max_head_buffer_size {
        return Err(RequestHeadParsingError::HeaderLineTooLong(line_buf).into());
      }

      let line = std::str::from_utf8(&line_buf)
        .map_err(|_| RequestHeadParsingError::HeaderLineIsNotUsAscii)?;

      if line == "\r\n" {
        break;
      }

      let line = line.strip_suffix("\r\n").ok_or(RequestHeadParsingError::HeaderLineNoCRLF)?;

      let mut line_parts = line.splitn(2, ": ");
      let name = unwrap_some(line_parts.next()).trim();

      if name.is_empty() {
        return Err(TiiError::from(RequestHeadParsingError::HeaderNameEmpty));
      }

      let value = line_parts.next().ok_or(RequestHeadParsingError::HeaderValueMissing)?.trim();

      if value.is_empty() {
        return Err(TiiError::from(RequestHeadParsingError::HeaderValueEmpty));
      }

      headers.add(HttpHeaderName::from(name), value);
    }

    let accept_hdr = headers.get(HttpHeaderName::Accept).unwrap_or("*/*"); //TODO This is probably also wrong.
    let accept = AcceptQualityMimeType::parse(accept_hdr);
    if accept.is_none() {
      // TODO should this be a hard error?
      warn_log!(
        "Request to '{}' has invalid Accept header '{}' will assume 'Accept: */*'",
        path.as_str(),
        accept_hdr
      );
    }

    let accept = accept.unwrap_or_else(|| vec![AcceptQualityMimeType::default()]);

    let content_type = headers.get(HttpHeaderName::ContentType).map(|ctype_raw| {
      let ctype = MimeType::parse_from_content_type_header(ctype_raw);
      if ctype.is_none() {
        warn_log!(
         "Request to '{}' has invalid Content-Type header '{}' will assume 'Content-Type: application/octet-stream'",
          path.as_str(),
          ctype_raw
        );
      }

      ctype.unwrap_or(MimeType::ApplicationOctetStream)
    });

    Ok(Self {
      method,
      path,
      query,
      version,
      headers,
      accept,
      content_type,
      status_line: status_line.to_string(),
    })
  }

  /// get the http version this request was made in by the client.
  pub fn get_version(&self) -> HttpVersion {
    self.version
  }

  /// Returns the raw status line.
  pub fn get_raw_status_line(&self) -> &str {
    self.status_line.as_str()
  }

  /// Returns the path the request will be routed to
  /// This should not contain any url encoding.
  pub fn get_path(&self) -> &str {
    self.path.as_str()
  }

  /// Sets the path the request will be routed to.
  /// This should not contain any url encoding.
  pub fn set_path(&mut self, path: impl ToString) {
    self.path = path.to_string();
  }

  /// Gets the query parameters.
  pub fn get_query(&self) -> &[(String, String)] {
    self.query.as_slice()
  }

  /// Gets the mutable Vec that contains the query parameters.
  pub fn query_mut(&mut self) -> &mut Vec<(String, String)> {
    &mut self.query
  }

  /// Set the query parameters
  pub fn set_query(&mut self, query: Vec<(String, String)>) {
    self.query = query;
  }

  /// Add a query parameter. Existing query parameters with the same key are not touched.
  pub fn add_query_param(&mut self, key: impl ToString, value: impl ToString) {
    self.query.push((key.to_string(), value.to_string()));
  }

  /// Removes all query parameters that match the given key.
  /// Returns the removed values.
  pub fn remove_query_params(&mut self, key: impl AsRef<str>) -> Vec<String> {
    let key = key.as_ref();

    let mut result = Vec::new();

    for n in (0..self.query.len()).rev() {
      let (k, _) = unwrap_some(self.query.get(n));
      if k == key {
        let (_, v) = self.query.remove(n);
        result.push(v);
      }
    }

    result.reverse();
    result
  }

  /// Removes all instances of the query parameter with the given key if there are any and adds a new query
  /// parameter with the given key and value to the end of the query parameters.
  ///
  /// If the key already has the value then its position is retained.
  /// All other values for the key are still removed.
  ///
  /// Returns the removed values.
  pub fn set_query_param(&mut self, key: impl ToString, value: impl ToString) -> Vec<String> {
    let key = key.to_string();
    let value = value.to_string();

    let mut result = Vec::new();
    let mut added = false;
    for n in (0..self.query.len()).rev() {
      let (k, v) = unwrap_some(self.query.get(n));
      if k == key.as_str() {
        if !added && v == value.as_str() {
          added = true;
          continue;
        }
        let (_, v) = self.query.remove(n);
        result.push(v);
      }
    }

    if !added {
      self.query.push((key, value));
    }

    result.reverse();
    result
  }

  /// Gets the first query parameter with the given key.
  pub fn get_query_param(&self, key: impl AsRef<str>) -> Option<&str> {
    let key = key.as_ref();
    for (k, v) in &self.query {
      if k == key {
        return Some(v.as_str());
      }
    }

    None
  }

  /// Gets all query params in order of appearance that contain the given key.
  /// Returns empty vec if the key doesn't exist.
  pub fn get_query_params(&self, key: impl AsRef<str>) -> Vec<&str> {
    let mut result = Vec::new();
    let key = key.as_ref();
    for (k, v) in &self.query {
      if k == key {
        result.push(v.as_str());
      }
    }

    result
  }

  /// gets the method of the request.
  pub fn get_method(&self) -> &HttpMethod {
    &self.method
  }

  /// Changes the method of the request
  pub fn set_method(&mut self, method: HttpMethod) {
    self.method = method;
  }

  /// Get the cookies from the request.
  pub fn get_cookies(&self) -> Vec<Cookie> {
    self
      .headers
      .get(HttpHeaderName::Cookie)
      .map(|cookies| {
        cookies
          .split(';')
          .filter_map(|cookie| {
            let (k, v) = cookie.split_once('=')?;
            Some(Cookie::new(k.trim(), v.trim()))
          })
          .collect()
      })
      .unwrap_or_default()
  }

  /// Attempts to get a specific cookie from the request.
  pub fn get_cookie(&self, name: impl AsRef<str>) -> Option<Cookie> {
    self.get_cookies().into_iter().find(|cookie| cookie.name == name.as_ref())
  }

  /// Manipulates the accept header values.
  /// This also overwrites the actual accept header!
  pub fn set_accept(&mut self, types: Vec<AcceptQualityMimeType>) {
    let hdr_value = AcceptQualityMimeType::elements_to_header_value(&types);
    self.headers.set(HttpHeaderName::Accept, hdr_value);
    self.accept = types;
  }

  /// Returns the content type of the body if any.
  /// This is usually equivalent to parsing the raw get_header() value of Content-Type.
  /// The only case where this is different is if the request as received from the network had an invalid Content-Type value then
  /// this value is ApplicationOctetStream even tho the raw header value is different.
  /// This returns none if the Content-Type header was not present at all.
  /// (For example ordinary GET requests do not have this header)
  pub fn get_content_type(&self) -> Option<&MimeType> {
    self.content_type.as_ref()
  }

  /// sets the content type header to given MimeType.
  /// This will affect both the header and the return value of get_content_type.
  pub fn set_content_type(&mut self, content_type: MimeType) {
    self.headers.set(HttpHeaderName::ContentType, content_type.as_str());
    self.content_type = Some(content_type);
  }

  /// Removes the content type header. get_content_type will return None after this call.
  pub fn remove_content_type(&mut self) -> Option<MimeType> {
    self.headers.remove(HttpHeaderName::ContentType);
    self.content_type.take()
  }

  /// Returns the acceptable mime types
  pub fn get_accept(&self) -> &[AcceptQualityMimeType] {
    self.accept.as_slice()
  }

  /// Returns an iterator over all headers.
  pub fn iter_headers(&self) -> impl Iterator<Item = &HttpHeader> {
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

  /// Removes all instances of a particular header.
  pub fn remove_headers(&mut self, hdr: impl AsRef<str>) -> TiiResult<()> {
    match &hdr.as_ref().into() {
      HttpHeaderName::Accept => {
        self.accept = vec![AcceptQualityMimeType::default()];
        self.headers.set(hdr, "*/*");
        Ok(())
      }
      HttpHeaderName::ContentType => {
        self.headers.remove(hdr);
        self.content_type = None;
        Ok(())
      }
      HttpHeaderName::TransferEncoding => {
        UserError::ImmutableRequestHeaderRemoved(HttpHeaderName::TransferEncoding).into()
      }
      HttpHeaderName::ContentLength => {
        UserError::ImmutableRequestHeaderRemoved(HttpHeaderName::ContentLength).into()
      }
      _ => {
        self.headers.remove(hdr);
        Ok(())
      }
    }
  }

  /// Sets the header value.
  /// Some header values cannot be modified through this fn and attempting to change them are a noop.
  pub fn set_header(&mut self, hdr: impl AsRef<str>, value: impl AsRef<str>) -> TiiResult<()> {
    let hdr_value = value.as_ref();
    match &hdr.as_ref().into() {
      HttpHeaderName::Accept => {
        if let Some(accept) = AcceptQualityMimeType::parse(hdr_value) {
          self.accept = accept;
          self.headers.set(hdr, value);
          return Ok(());
        }

        UserError::IllegalAcceptHeaderValueSet(hdr_value.to_string()).into()
      }
      HttpHeaderName::ContentType => {
        let mime = MimeType::parse(hdr_value)
          .ok_or_else(|| UserError::IllegalContentTypeHeaderValueSet(hdr_value.to_string()))?;
        self.headers.set(HttpHeaderName::ContentType, hdr_value);
        self.content_type = Some(mime);
        Ok(())
      }
      HttpHeaderName::TransferEncoding => UserError::ImmutableRequestHeaderModified(
        HttpHeaderName::TransferEncoding,
        hdr_value.to_string(),
      )
      .into(),
      HttpHeaderName::ContentLength => UserError::ImmutableRequestHeaderModified(
        HttpHeaderName::ContentLength,
        hdr_value.to_string(),
      )
      .into(),
      _ => {
        self.headers.set(hdr, value);
        Ok(())
      }
    }
  }

  /// Adds a new header value to the headers. This can be the first value with the given key or an additional value.
  pub fn add_header(&mut self, hdr: impl AsRef<str>, value: impl AsRef<str>) -> TiiResult<()> {
    let hdr_value = value.as_ref();
    match &hdr.as_ref().into() {
      HttpHeaderName::Accept => {
        if let Some(accept) = AcceptQualityMimeType::parse(hdr_value) {
          if let Some(old_value) = self.headers.try_set(hdr, hdr_value) {
            return UserError::MultipleAcceptHeaderValuesSet(
              old_value.to_string(),
              hdr_value.to_string(),
            )
            .into();
          }
          self.accept = accept;
          return Ok(());
        }
        UserError::IllegalAcceptHeaderValueSet(hdr_value.to_string()).into()
      }
      HttpHeaderName::ContentType => {
        let mime = MimeType::parse(hdr_value)
          .ok_or_else(|| UserError::IllegalContentTypeHeaderValueSet(hdr_value.to_string()))?;
        if let Some(old_value) = self.headers.try_set(hdr, hdr_value) {
          return UserError::MultipleContentTypeHeaderValuesSet(
            old_value.to_string(),
            hdr_value.to_string(),
          )
          .into();
        }
        self.content_type = Some(mime);
        Ok(())
      }
      HttpHeaderName::TransferEncoding => UserError::ImmutableRequestHeaderModified(
        HttpHeaderName::TransferEncoding,
        hdr_value.to_string(),
      )
      .into(),
      HttpHeaderName::ContentLength => UserError::ImmutableRequestHeaderModified(
        HttpHeaderName::ContentLength,
        hdr_value.to_string(),
      )
      .into(),
      _ => {
        self.headers.add(hdr, value);
        Ok(())
      }
    }
  }
}
