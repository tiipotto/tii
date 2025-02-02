use crate::mock_stream::MockStream;
use tii::http::mime::MimeType;
use tii::http::request::HttpVersion;
use tii::http::request_context::RequestContext;
use tii::http::response_body::ResponseBody;
use tii::http::Response;
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;
use std::sync::atomic::{AtomicUsize, Ordering};

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);
fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  COUNTER.fetch_add(1, Ordering::SeqCst);
  assert_eq!(HttpVersion::Http11, ctx.request_head().version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));
  let mut body = Vec::new();
  ctx.request_body().unwrap().read_to_end(&mut body)?;
  assert_eq!(body.as_slice(), b"ABCDEF");

  Ok(Response::ok(ResponseBody::from_slice("Okay!"), MimeType::TextPlain))
}

#[test]
pub fn tc24() {
  let server = TiiBuilder::builder(|builder| {
    builder.router(|rt| {
      rt.post("/dummy")
        .consumes(MimeType::TextPlain)
        .produces(MimeType::TextPlain)
        .endpoint(dummy_route)
    })
  })
  .expect("ERROR");

  let stream = MockStream::with_str("POST /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain\r\nContent-Type: text/plain\r\nContent-Length: 6\r\n\r\nABCDEF");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");

  let stream = MockStream::with_str("POST /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain\r\nContent-Type: text/rtf\r\nContent-Length: 6\r\n\r\nABCDEF");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 415 Unsupported Media Type\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n"
  );

  let stream = MockStream::with_str("POST /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/rtf\r\nContent-Type: text/plain\r\nContent-Length: 6\r\n\r\nABCDEF");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 406 Not Acceptable\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain\r\nContent-Type: text/plain\r\nContent-Length: 6\r\n\r\nABCDEF");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 405 Method Not Allowed\r\nAllow: POST\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");

  assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
}
