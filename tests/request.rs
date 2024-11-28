mod mock_stream;

use crate::mock_stream::MockStream;
use humpty::http::cookie::Cookie;
use humpty::http::headers::{Header, HeaderName};
use humpty::http::method::Method;
use humpty::http::RequestHead;

use humpty::http::request::HttpVersion;
use humpty::stream::IntoConnectionStream;
use std::collections::VecDeque;
use std::iter::FromIterator;

#[allow(deprecated)]
#[test]
fn test_request_from_stream() {
  let test_data = b"GET /testpath?foo=bar HTTP/1.1\r\nHost: localhost\r\n\r\n";
  let stream = MockStream::with_data(VecDeque::from_iter(test_data.iter().cloned()));
  let raw_stream = stream.clone().into_connection_stream();

  let request = RequestHead::new(raw_stream.as_ref(), 8096);

  let request = request.unwrap();
  let expected_uri: String = "/testpath".into();
  assert_eq!(request.method(), &Method::Get);
  assert_eq!(request.path(), expected_uri);
  assert_eq!(request.query(), &[("foo".to_string(), "bar".to_string())]);
  assert_eq!(request.version(), HttpVersion::Http11);

  let mut expected_headers = Vec::new();
  expected_headers.push(Header::new(HeaderName::Host, "localhost"));

  let collected_headers = request.get_all_headers().map(|a| a.clone()).collect::<Vec<_>>();
  assert_eq!(collected_headers, expected_headers);
}

#[test]
fn test_cookie_request() {
  let test_data = b"GET / HTTP/1.1\r\nHost: localhost\r\nCookie: foo=bar; baz=qux\r\n\r\n";
  let stream = MockStream::with_data(VecDeque::from_iter(test_data.iter().cloned()));
  let raw_stream = stream.clone().into_connection_stream();
  let request = RequestHead::new(raw_stream.as_ref(), 8096).unwrap();

  let mut expected_cookies = vec![Cookie::new("foo", "bar"), Cookie::new("baz", "qux")];

  assert_eq!(request.get_cookies(), expected_cookies);

  assert_eq!(request.get_cookie("baz"), expected_cookies.pop());
  assert_eq!(request.get_cookie("foo"), expected_cookies.pop());
  assert_eq!(request.get_cookie("sus"), None);
}

#[test]
fn test_proxied_request_from_stream() {
  let test_data =
    b"GET /testpath HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-For: 9.10.11.12,13.14.15.16\r\n\r\n";
  let stream = MockStream::with_data(VecDeque::from_iter(test_data.iter().cloned()));
  let raw_stream = stream.clone().into_connection_stream();

  let request = RequestHead::new(raw_stream.as_ref(), 8096);

  let request = request.unwrap();
  let expected_uri: String = "/testpath".into();
  assert_eq!(request.method(), &Method::Get);
  assert_eq!(request.path(), expected_uri);
  assert_eq!(request.version(), HttpVersion::Http11);

  let mut expected_headers = Vec::new();
  expected_headers.push(Header::new(HeaderName::Host, "localhost"));
  expected_headers.push(Header::new("X-Forwarded-For", "9.10.11.12,13.14.15.16"));
  let collected: Vec<Header> = request.get_all_headers().map(|h| h.clone()).collect();

  assert_eq!(collected, expected_headers);
}
