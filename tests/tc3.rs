use crate::mock_stream::MockStream;
use std::io;
use std::sync::atomic::AtomicUsize;
use tii::HttpVersion;
use tii::RequestContext;
use tii::ResponseBody;
use tii::ServerBuilder;
use tii::TiiResult;
use tii::{Response, StatusCode};

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http11, ctx.get_version());
  assert_eq!(ctx.get_header("Hdr"), Some("test"));

  Response::new(StatusCode::OK)
    .with_body(ResponseBody::from_slice("Okay!"))
    .with_header("Connection", "Close")
}

#[test]
pub fn tc3() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
  assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
  assert_eq!(err.to_string(), "Endpoint has set banned header 'Connection'");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
