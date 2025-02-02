use crate::mock_stream::MockStream;
use tii::http::request::HttpVersion;
use tii::http::request_context::RequestContext;
use tii::http::{Response, StatusCode};
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;
use std::sync::atomic::AtomicUsize;

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http11, ctx.request_head().version());
  Response::new(StatusCode::OK).with_body("Okay!").with_header("Fubar", "Dubar")
}

#[test]
pub fn tc20() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

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
