use crate::mock_stream::MockStream;
use humpty::http::request::HttpVersion;
use humpty::http::request_context::RequestContext;
use humpty::http::{Response, StatusCode};
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use std::sync::atomic::AtomicUsize;

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn dummy_route(ctx: &RequestContext) -> HumptyResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http11, ctx.request_head().version);
  Response::new(StatusCode::OK).with_body("Okay!").with_header("Fubar", "Dubar").into()
}

#[test]
pub fn tc20() {
  let server = HumptyBuilder::default().router(|rt| rt.with_route("/dummy", dummy_route)).build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 200 OK\r\nFubar: Dubar\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!"
  );
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
}
