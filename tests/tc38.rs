use crate::mock_stream::MockStream;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiResult;
use tii::{MimeType, RequestContext, RequestHeadParsingError, TiiError};

mod mock_stream;

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  Ok(Response::ok(format!("Okay! {}", ctx.request_head().get_path()), MimeType::TextPlain))
}

#[test]
pub fn tc38() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy*$\\'()+,:;@[]-_~ HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 28\r\n\r\nOkay! /dummy*$\\'()+,:;@[]-_~");
}

#[test]
pub fn tc38_b() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy#1 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert!(matches!(
    err,
    TiiError::RequestHeadParsing(RequestHeadParsingError::StatusLineContainsInvalidBytes)
  ));
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
