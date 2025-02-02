use crate::mock_stream::MockStream;
use std::io;
use std::io::ErrorKind;
use tii::http::mime::MimeType;
use tii::http::request_context::RequestContext;
use tii::http::Response;
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;

mod mock_stream;

fn dummy_route_invalid_data(ctx: &RequestContext) -> TiiResult<Response> {
  let body = ctx.request_body().unwrap();
  let mut data = vec![];
  //The body trailer is malformed, we will eat the error.
  //This MUST CAUSE error
  let err = body.read_to_end(&mut data).unwrap_err();
  assert_eq!(err.kind(), ErrorKind::InvalidData);
  Response::ok("Okay!", MimeType::TextPlain).into()
}

fn dummy_route_eof(ctx: &RequestContext) -> TiiResult<Response> {
  let body = ctx.request_body().unwrap();
  let mut data = vec![];
  //The body trailer is malformed, we will eat the error.
  //This MUST CAUSE error
  let err = body.read_to_end(&mut data).unwrap_err();
  assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
  Response::ok("Okay!", MimeType::TextPlain).into()
}

#[test]
pub fn tc21a() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERR")
    .build();
  // INVALID Chunked trailer
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\n\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21b() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERR")
    .build();
  // INVALID Chunked trailer
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\r\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21c() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERR")
    .build();
  // INVALID Chunked trailer
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\n\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21d() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERR")
    .build();
  // INVALID Chunked trailer
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\r");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21e() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\n\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21f() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\r10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21g() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame length
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21h() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_eof))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame EOF
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21i() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame length
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n0000000000000000005\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21j() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame length
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\nzxi1\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21k() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_eof))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame length
  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n1234",
  );
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}

#[test]
pub fn tc21l() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route_invalid_data))
    .expect("ERROR")
    .build();
  // INVALID Chunked frame length
  let n = "GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5";
  let n2 = "\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n";
  let mut n3: Vec<u8> = Vec::new();
  n3.extend_from_slice(n.as_bytes());
  n3.extend_from_slice(&[0b1011_1111]);
  n3.extend_from_slice(n2.as_bytes());

  let stream = MockStream::with_slice(n3.as_slice());
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  let err = err.downcast_ref::<io::Error>().unwrap();
  assert_eq!(err.kind(), ErrorKind::BrokenPipe);
}
