use crate::mock_stream::MockStream;
use tii::http::mime::MimeType;
use tii::http::request_context::RequestContext;
use tii::http::Response;
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;
use std::io;

mod mock_stream;

fn dummy_route(_ctx: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}

fn dummy_route2(ctx: &RequestContext) -> Response {
  Response::ok(format!("{:?}", ctx.request_head().query()), MimeType::TextPlain)
}

#[test]
pub fn tc35_1() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?bla=xxxx=yyyy HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "InvalidQueryString(\"bla=xxxx=yyyy\")");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc35_2() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?&b HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "InvalidQueryString(\"&b\")");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc35_3() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?a=%BF HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "InvalidQueryString(\"a=%BF\")");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc35_4() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?a=? HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "InvalidQueryString(\"a=?\")");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc35_5() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?a=a&b=%BF HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "InvalidQueryString(\"a=a&b=%BF\")");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc35_6() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route2))
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("GET /dummy?a!=!&b!=a! HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 27\r\n\r\n[(\"a!\", \"!\"), (\"b!\", \"a!\")]");
}
