use crate::mock_stream::MockStream;
use std::io::ErrorKind;
use tii::ServerBuilder;
use tii::MimeType;
use tii::RequestContext;
use tii::Response;
use tii::TiiResult;

mod mock_stream;

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  let body = ctx.request_body().unwrap();
  assert_eq!(0, body.read(&mut [])?);
  let mut data = [0; 12];
  if let Some(rem) = body.remaining()? {
    assert_eq!(21, rem);
  }
  body.read_exact(data.as_mut())?;
  if let Some(rem) = body.remaining()? {
    assert_eq!(9, rem);
  }

  assert_eq!("123451234567", std::str::from_utf8(&data).expect("ERR"));
  let mut data = [0; 9];
  body.read_exact(data.as_mut())?;
  assert_eq!("890123456", std::str::from_utf8(&data).expect("ERR"));
  if let Some(rem) = body.remaining()? {
    assert_eq!(0, rem);
  }
  let err = body.read_exact(data.as_mut()).unwrap_err();
  assert_eq!(ErrorKind::UnexpectedEof, err.kind());
  Response::ok("Okay!", MimeType::TextPlain).into()
}

#[test]
pub fn tc22a() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERROR").build();
  // INVALID Chunked trailer
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: 5\r\n\r\nOkay!");
}

#[test]
pub fn tc22b() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERROR").build();
  // INVALID Chunked trailer
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nContent-Length: 21\r\n\r\n123451234567890123456");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: 5\r\n\r\nOkay!");
}
