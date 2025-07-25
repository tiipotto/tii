mod mock_stream;

use crate::mock_stream::MockStream;
use tii::{
  AcceptQualityMimeType, Cookie, MimeType, QValue, RequestContext, RequestHeadParsingError,
  TiiError, UserError,
};
use tii::{HttpHeader, HttpHeaderName};
use tii::{HttpMethod, TypeSystem};

use std::collections::VecDeque;
use std::iter::FromIterator;
use std::vec;
use tii::HttpVersion;
use tii::IntoConnectionStream;

#[allow(deprecated)]
#[test]
fn test_request_from_stream() {
  let test_data = b"GET /testpath?foo=bar HTTP/1.1\r\nHost: localhost\r\n\r\n";
  let stream = MockStream::with_data(VecDeque::from_iter(test_data.iter().cloned()));
  let raw_stream = stream.clone().into_connection_stream();

  let request = RequestContext::read(raw_stream.as_ref(), None, 8096, TypeSystem::empty());

  let request = request.unwrap();
  let expected_uri: String = "/testpath".into();
  assert_eq!(request.get_method(), &HttpMethod::Get);
  assert_eq!(request.get_path(), expected_uri);
  assert_eq!(request.get_query(), &[("foo".to_string(), "bar".to_string())]);
  assert_eq!(request.get_version(), HttpVersion::Http11);

  let mut expected_headers = Vec::new();
  expected_headers.push(HttpHeader::new(HttpHeaderName::Host, "localhost"));

  let collected_headers = request.iter_headers().cloned().collect::<Vec<_>>();
  assert_eq!(collected_headers, expected_headers);
}

#[test]
fn test_cookie_request() {
  let test_data = b"GET / HTTP/1.1\r\nHost: localhost\r\nCookie: foo=bar; baz=qux\r\n\r\n";
  let stream = MockStream::with_data(VecDeque::from_iter(test_data.iter().cloned()));
  let raw_stream = stream.clone().into_connection_stream();
  let request = RequestContext::read(raw_stream.as_ref(), None, 8096, TypeSystem::empty()).unwrap();

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

  let request = RequestContext::read(raw_stream.as_ref(), None, 8096, TypeSystem::empty());

  let request = request.unwrap();
  let expected_uri: String = "/testpath".into();
  assert_eq!(request.get_method(), &HttpMethod::Get);
  assert_eq!(request.get_path(), expected_uri);
  assert_eq!(request.get_version(), HttpVersion::Http11);

  let mut expected_headers = Vec::new();
  expected_headers.push(HttpHeader::new(HttpHeaderName::Host, "localhost"));
  expected_headers.push(HttpHeader::new("X-Forwarded-For", "9.10.11.12,13.14.15.16"));
  let collected: Vec<HttpHeader> = request.iter_headers().cloned().collect();

  assert_eq!(collected, expected_headers);
}

#[test]
fn test_mock_request_head() {
  let mock_head = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "/beep",
    vec![("bep", "bop")],
    Vec::new(),
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap();
  assert_eq!(mock_head.get_raw_status_line(), "GET /beep?bep=bop HTTP/1.1");
  let mock_head = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "/beepä/bo#ö!/",
    vec![("bepä", "büop")],
    Vec::new(),
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap();
  assert_eq!(
    mock_head.get_raw_status_line(),
    "GET /beep%C3%A4/bo%23%C3%B6%21/?bep%C3%A4=b%C3%BCop HTTP/1.1"
  );
  let mock_head = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "/beep/bop/",
    vec![("", "büop"), ("", "nop"), ("cop", "")],
    Vec::new(),
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap();
  assert_eq!(mock_head.get_raw_status_line(), "GET /beep/bop/?=b%C3%BCop&=nop&cop HTTP/1.1");
  let err = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "beep/bop/",
    vec![("", "büop"), ("", "nop"), ("cop", "")],
    Vec::new(),
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidPath(s)) => {
      assert_eq!(s, "beep/bop/")
    }
    _ => panic!("Unexpected error {err}"),
  }

  let mock_head = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http09,
    "/beep/bop/",
    vec![("mep", "mop")],
    Vec::new(),
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap();
  assert_eq!(mock_head.get_raw_status_line(), "GET /beep/bop/?mep=mop");

  let err = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Post,
    HttpVersion::Http09,
    "/beep/bop",
    Vec::<(&str, &str)>::new(),
    Vec::new(),
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::MethodNotSupportedByHttpVersion(
      v,
      m,
    )) => assert_eq!((v, m), (HttpVersion::Http09, HttpMethod::Post)),
    _ => panic!("Unexpected error {err}"),
  }

  let err = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http09,
    "/beep/bop",
    Vec::<(&str, &str)>::new(),
    vec![HttpHeader::new("abc", "def")],
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap_err();
  match err {
    TiiError::UserError(UserError::HeaderNotSupportedByHttpVersion(v)) => {
      assert_eq!(v, HttpVersion::Http09)
    }
    _ => panic!("Unexpected error {err}"),
  }

  let err = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "/beep/bop",
    Vec::<(&str, &str)>::new(),
    vec![HttpHeader::new("Accept", "*//")],
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap_err();
  match err {
    TiiError::UserError(UserError::IllegalAcceptHeaderValueSet(v)) => assert_eq!(v, "*//"),
    _ => panic!("Unexpected error {err}"),
  }

  let err = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "/beep/bop",
    Vec::<(&str, &str)>::new(),
    vec![HttpHeader::new("Content-Type", "*//")],
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap_err();
  match err {
    TiiError::UserError(UserError::IllegalContentTypeHeaderValueSet(v)) => assert_eq!(v, "*//"),
    _ => panic!("Unexpected error {err}"),
  }

  let mut mock_head = RequestContext::new(
    0,
    "localhost",
    "localhost",
    HttpMethod::Get,
    HttpVersion::Http11,
    "/beep/bop",
    Vec::<(&str, &str)>::new(),
    vec![
      HttpHeader::new("Content-Type", "text/plain"),
      HttpHeader::new("Accept", "application/json"),
    ],
    None,
    None,
    TypeSystem::empty(),
  )
  .unwrap();
  assert_eq!(
    mock_head.get_accept().first(),
    Some(&AcceptQualityMimeType::from_mime(MimeType::ApplicationJson, QValue::default()))
  );
  assert_eq!(mock_head.get_content_type(), Some(&MimeType::TextPlain));
  mock_head.set_content_type(MimeType::Application7Zip);
  assert_eq!(mock_head.get_content_type(), Some(&MimeType::Application7Zip));
  assert_eq!(mock_head.get_header("Content-Type"), Some(MimeType::Application7Zip.as_str()));
  mock_head.remove_headers("Content-Type").unwrap();
  assert_eq!(mock_head.get_header("Content-Type"), None);
  assert_eq!(mock_head.get_content_type(), None);
  mock_head.remove_headers("Accept").unwrap();
  assert_eq!(mock_head.get_header("Accept"), Some("*/*"));
  assert_eq!(
    mock_head.get_accept().first(),
    Some(&AcceptQualityMimeType::wildcard(QValue::default()))
  );
  mock_head.set_header("meep", "moop").unwrap();
  assert_eq!(mock_head.get_header("meep"), Some("moop"));
  mock_head.remove_headers("meep").unwrap();
  assert_eq!(mock_head.get_header("meep"), None);
  mock_head.set_content_type(MimeType::Application7Zip);
  mock_head.set_accept(vec![MimeType::ApplicationJson.into_accept(QValue::default())]);
  let err = mock_head.add_header(HttpHeaderName::ContentType, "application/zip").unwrap_err();
  match err {
    TiiError::UserError(UserError::MultipleContentTypeHeaderValuesSet(v1, v2)) => assert_eq!(
      (v1, v2),
      ("application/x-7z-compressed".to_string(), "application/zip".to_string())
    ),
    _ => panic!("Unexpected error {err}"),
  }
  let err = mock_head.add_header(HttpHeaderName::Accept, "*/*").unwrap_err();
  match err {
    TiiError::UserError(UserError::MultipleAcceptHeaderValuesSet(v1, v2)) => {
      assert_eq!((v1, v2), ("application/json".to_string(), "*/*".to_string()))
    }
    _ => panic!("Unexpected error {err}"),
  }

  let err = mock_head.remove_headers(HttpHeaderName::ContentLength).unwrap_err();
  match err {
    TiiError::UserError(UserError::ImmutableRequestHeaderRemoved(v)) => {
      assert_eq!(v, HttpHeaderName::ContentLength)
    }
    _ => panic!("Unexpected error {err}"),
  }
  let err = mock_head.remove_headers(HttpHeaderName::TransferEncoding).unwrap_err();
  match err {
    TiiError::UserError(UserError::ImmutableRequestHeaderRemoved(v)) => {
      assert_eq!(v, HttpHeaderName::TransferEncoding)
    }
    _ => panic!("Unexpected error {err}"),
  }
  let err = mock_head.set_header(HttpHeaderName::TransferEncoding, "bup").unwrap_err();
  match err {
    TiiError::UserError(UserError::ImmutableRequestHeaderModified(v, l)) => {
      assert_eq!((v, l), (HttpHeaderName::TransferEncoding, "bup".to_string()))
    }
    _ => panic!("Unexpected error {err}"),
  }
  let err = mock_head.add_header(HttpHeaderName::TransferEncoding, "bup").unwrap_err();
  match err {
    TiiError::UserError(UserError::ImmutableRequestHeaderModified(v, l)) => {
      assert_eq!((v, l), (HttpHeaderName::TransferEncoding, "bup".to_string()))
    }
    _ => panic!("Unexpected error {err}"),
  }
  let err = mock_head.set_header(HttpHeaderName::ContentLength, "bup").unwrap_err();
  match err {
    TiiError::UserError(UserError::ImmutableRequestHeaderModified(v, l)) => {
      assert_eq!((v, l), (HttpHeaderName::ContentLength, "bup".to_string()))
    }
    _ => panic!("Unexpected error {err}"),
  }
  let err = mock_head.add_header(HttpHeaderName::ContentLength, "bup").unwrap_err();
  match err {
    TiiError::UserError(UserError::ImmutableRequestHeaderModified(v, l)) => {
      assert_eq!((v, l), (HttpHeaderName::ContentLength, "bup".to_string()))
    }
    _ => panic!("Unexpected error {err}"),
  }
}
