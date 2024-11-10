use crate::mock_stream::MockStream;
use humpty::http::request::HttpVersion;
use humpty::http::request_context::RequestContext;
use humpty::http::response_body::ResponseBody;
use humpty::http::{Response, StatusCode};
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use std::io;
use std::sync::atomic::AtomicUsize;

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn dummy_route(ctx: &RequestContext) -> HumptyResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http11, ctx.request_head().version);
  assert_eq!(ctx.request_head().headers.get("Hdr"), Some("test"));

  Ok(
    Response::new(StatusCode::OK)
      .with_body(ResponseBody::from_slice("Okay!"))
      .with_header("Connection", "Close"),
  )
}

#[test]
pub fn tc3() {
  let server = HumptyBuilder::default().router(|rt| rt.with_route("/dummy", dummy_route)).build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
  assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
  assert_eq!(err.to_string(), "Endpoint has set banned header 'Connection'");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
