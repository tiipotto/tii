use crate::mock_stream::MockStream;
use humpty::http::mime::MimeType;
use humpty::http::request::HttpVersion;
use humpty::http::request_context::RequestContext;
use humpty::http::response_body::ResponseBody;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use std::sync::atomic::{AtomicUsize, Ordering};

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);
fn dummy_route(ctx: &RequestContext) -> HumptyResult<Response> {
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
  let server = HumptyBuilder::default()
    .router(|rt| {
      rt.post("/dummy")
        .consumes(MimeType::TextPlain)
        .produces(MimeType::TextPlain)
        .endpoint(dummy_route)
    })
    .build();

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
