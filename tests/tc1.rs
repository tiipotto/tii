use crate::mock_stream::MockStream;
use humpty::http::method::Method;
use humpty::http::request::HttpVersion;
use humpty::http::request_context::RequestContext;
use humpty::http::response_body::ResponseBody;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use std::sync::atomic::AtomicU64;

mod mock_stream;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn dummy_route(ctx: &RequestContext) -> HumptyResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http09, ctx.request_head().version);
  assert!(ctx.request_head().headers.is_empty());
  let hdr_clone = ctx.request_head().clone();
  assert!(hdr_clone.headers.is_empty());
  assert_eq!(hdr_clone.status_line, "GET /dummy");
  assert_eq!(hdr_clone.version, HttpVersion::Http09);
  assert_eq!(hdr_clone.path, "/dummy");
  assert_eq!(hdr_clone.method, Method::Get);
  assert_eq!(hdr_clone.query, "");
  Ok(Response::ok(ResponseBody::from_slice("Okay!")))
}

#[test]
pub fn tc1() {
  let server = HumptyBuilder::default().router(|rt| rt.with_route("/dummy", dummy_route)).build();

  let stream = MockStream::with_str("GET /dummy\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "Okay!");
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
}
