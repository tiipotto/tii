//! Provides functionality for handling HTTP requests.

use crate::http::cookie::Cookie;
use crate::http::headers::{HeaderType, Headers};
use crate::http::method::Method;

use crate::stream::ConnectionStream;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::ErrorKind;

/// Enum for http versions humpty supports.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum HttpVersion {
  /// Earliest http version. Has no concept of request bodies or headers. to trigger a request run `echo -ne 'GET /path/goes/here\r\n' | nc 127.0.0.1 8080`
  /// Responses are just the body, no headers, no nothing.
  Http09,
  /// First actually usable http version. Has headers, bodies, etc but notably 1 connection per request and thus no transfer encoding
  Http10,
  /// Most recent 1.X version, has all features.
  Http11,
}

impl HttpVersion {
  pub(crate) fn as_bytes(&self) -> &[u8] {
    match self {
      HttpVersion::Http09 => &[],
      HttpVersion::Http10 => b"HTTP/1.0",
      HttpVersion::Http11 => b"HTTP/1.1",
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
  fn try_from(value: &str) -> Result<Self, &str> {
    match value {
      "HTTP/1.0" => Ok(HttpVersion::Http10),
      "HTTP/1.1" => Ok(HttpVersion::Http11),
      _ => Err(value),
    }
  }
}

/// Represents a request to the server.
/// Contains parsed information about the request's data.
//TODO make it harder/impossible to put this struct into an illegal state.
#[derive(Clone, Debug)]
pub struct RequestHead {
  /// The method used in making the request, e.g. "GET".
  pub method: Method,

  /// The HTTP version of the request.
  pub version: HttpVersion,

  /// The status line as is.
  /// For example "GET /index.html HTTP1.1"
  /// the crlf has been stripped already!
  pub status_line: String,

  /// The path to which the request was made.
  pub path: String,

  /// The raw query string of the request.
  pub query: String,

  /// Vec of query parameters, key=value in order of appearance.
  //TODO implement this
  //pub query_params: Vec<(String, String)>,

  /// A list of headers included in the request.
  pub headers: Headers,
}

impl RequestHead {
  /// Attempts to read and parse one HTTP request from the given reader.
  pub fn new(stream: &dyn ConnectionStream) -> io::Result<Self> {
    let mut start_line_buf: Vec<u8> = Vec::with_capacity(256);
    //TODO fix ddos potential here, limit read to 64k or some other reasonable size.
    //Possible attack on this is to just write ~Mem amount of data and then just keep
    //drip feeding us 1 byte of data every so often to deny memory to actual requests.
    stream.read_until(0xA, &mut start_line_buf)?;

    let start_line_string =
        // TODO this must be US-ASCII not utf-8!
        std::str::from_utf8(&start_line_buf).map_err(|_| io::Error::new(ErrorKind::Other, "status line is not valid US-ASCII"))?;

    let status_line = start_line_string
      .strip_suffix("\r\n")
      .ok_or_else(|| io::Error::new(ErrorKind::Other, "status line did not end with CRLF"))?;

    let mut start_line = status_line.split(' ');

    let method = Method::from_name(start_line.next().ok_or_else(|| {
      io::Error::new(ErrorKind::Other, "status line did not contain ' ' (0x32) bytes")
    })?);

    let mut uri_iter = start_line
      .next()
      .ok_or_else(|| {
        io::Error::new(ErrorKind::Other, "status line did not contain ' ' (0x32) bytes")
      })?
      .splitn(2, '?');

    let version = start_line
      .next()
      .map(HttpVersion::try_from)
      .unwrap_or(Ok(HttpVersion::Http09)) //Http 0.9 has no suffix
      .map_err(|version| {
        io::Error::new(
          ErrorKind::InvalidData,
          format!("The http version {version} is not supported."),
        )
      })?;

    if start_line.next().is_some() {
      return Err(io::Error::new(
        ErrorKind::InvalidData,
        "The request status line contains more than two ' ' (0x32) bytes.",
      ));
    }

    let uri = uri_iter.next().unwrap().to_string();
    let query = uri_iter.next().unwrap_or("").to_string();

    let mut headers = Headers::new();

    if version == HttpVersion::Http09 {
      if method != Method::Get {
        return Err(io::Error::new(
          ErrorKind::InvalidData,
          format!("HTTP 0.9 only supports GET but method was {method}"),
        ));
      }

      return Ok(Self {
        method,
        path: uri,
        query,
        version,
        headers,
        status_line: status_line.to_string(),
      });
    }

    loop {
      let mut line_buf: Vec<u8> = Vec::with_capacity(256);
      stream.read_until(0xA, &mut line_buf)?;
      let line = std::str::from_utf8(&line_buf)
        .map_err(|_| io::Error::new(ErrorKind::Other, "header line is not valid US-ASCII"))?;

      if line == "\r\n" {
        break;
      } else {
        safe_assert(line.len() >= 2)?;
        let line_without_crlf = &line[0..line.len() - 2];
        let mut line_parts = line_without_crlf.splitn(2, ':');
        headers.add(
          HeaderType::from(
            line_parts.next().ok_or_else(|| {
              io::Error::new(ErrorKind::InvalidData, "Malformed http header name")
            })?,
          ),
          line_parts
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "Malformed http header value"))?
            .trim_start(),
        );
      }
    }

    Ok(Self { method, path: uri, query, version, headers, status_line: status_line.to_string() })
  }

  /// Get the cookies from the request.
  pub fn get_cookies(&self) -> Vec<Cookie> {
    self
      .headers
      .get(HeaderType::Cookie)
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
}

/// Asserts that the condition is true, returning a `Result`.
fn safe_assert(condition: bool) -> io::Result<()> {
  match condition {
    true => Ok(()),
    false => Err(io::Error::new(ErrorKind::InvalidData, "Assertion failed")),
  }
}
