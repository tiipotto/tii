use crate::mock_stream::MockStream;
use std::sync::atomic::AtomicUsize;
use tii::ServerBuilder;
use tii::HttpVersion;
use tii::RequestContext;
use tii::ResponseBody;
use tii::TiiResult;
use tii::{Response, StatusCode};

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http11, ctx.request_head().get_version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));

  Ok(Response::new(StatusCode::OK).with_body(ResponseBody::from_slice("Okay!")))
}

#[test]
pub fn tc2() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
}
