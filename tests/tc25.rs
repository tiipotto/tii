use crate::mock_stream::MockStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use tii::TiiBuilder;
use tii::TiiHttpVersion;
use tii::TiiMimeType;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::TiiResponseBody;
use tii::TiiResult;

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);
fn dummy_route(ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  COUNTER.fetch_add(1, Ordering::SeqCst);
  assert_eq!(TiiHttpVersion::Http11, ctx.request_head().get_version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));
  let mut body = Vec::new();
  ctx.request_body().unwrap().read_to_end(&mut body)?;
  assert_eq!(body.as_slice(), b"ABCDEF");

  Ok(TiiResponse::ok(TiiResponseBody::from_slice("Okay!"), TiiMimeType::TextPlain))
}

static COUNTER2: AtomicUsize = AtomicUsize::new(0);
fn dummy_route2(ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  COUNTER2.fetch_add(1, Ordering::SeqCst);
  assert_eq!(TiiHttpVersion::Http11, ctx.request_head().get_version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));
  let mut body = Vec::new();
  ctx.request_body().unwrap().read_to_end(&mut body)?;
  assert_eq!(body.as_slice(), b"ABCDEF");

  Ok(TiiResponse::ok(TiiResponseBody::from_slice("Nice!"), TiiMimeType::TextPlain))
}

#[test]
pub fn tc25() {
  let server = TiiBuilder::default()
    .router(|rt| {
      rt.post("/dummy")
        .consumes(TiiMimeType::TextPlain)
        .produces(TiiMimeType::TextPlain)
        .endpoint(dummy_route)?
        .post("/dummy")
        .consumes(TiiMimeType::TextCsv)
        .produces(TiiMimeType::TextPlain)
        .endpoint(dummy_route2)
    })
    .expect("ERROR")
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

  let stream = MockStream::with_str("POST /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain\r\nContent-Type: text/csv\r\nContent-Length: 6\r\n\r\nABCDEF");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nNice!");

  assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
  assert_eq!(COUNTER2.load(Ordering::SeqCst), 1);
}
