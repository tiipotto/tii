use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, RequestHeadParsingError, Response, ServerBuilder, TiiError};

mod mock_stream;
fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}
#[test]
pub fn tc52() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?quer%80=2 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidQueryString(n)) => {
      assert_eq!(n, "quer%80=2");
    }
    _ => panic!("unexpected error: {:?}", err),
  }
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc52_b() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?abac=blar&quer%80=2 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidQueryString(n)) => {
      assert_eq!(n, "abac=blar&quer%80=2");
    }
    _ => panic!("unexpected error: {:?}", err),
  }
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc52_c() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?abac=blar&quer%80=2&bum=ba HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidQueryString(n)) => {
      assert_eq!(n, "abac=blar&quer%80=2&bum=ba");
    }
    _ => panic!("unexpected error: {:?}", err),
  }
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc52_d() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?abac=blar&quer=2%80&bum=ba HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidQueryString(n)) => {
      assert_eq!(n, "abac=blar&quer=2%80&bum=ba");
    }
    _ => panic!("unexpected error: {:?}", err),
  }
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
