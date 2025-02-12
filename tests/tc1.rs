use crate::mock_stream::MockStream;
use std::sync::atomic::AtomicUsize;
use tii::ServerBuilder;
use tii::HttpMethod;
use tii::HttpVersion;
use tii::RequestContext;
use tii::ResponseBody;
use tii::TiiResult;
use tii::{Response, StatusCode};

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[allow(deprecated)]
fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(HttpVersion::Http09, ctx.request_head().get_version());
  assert!(ctx.request_head().iter_headers().next().is_none());
  let hdr_clone = ctx.request_head().clone();
  assert!(hdr_clone.iter_headers().next().is_none());
  assert_eq!(hdr_clone.get_raw_status_line(), "GET /dummy");
  assert_eq!(hdr_clone.get_version(), HttpVersion::Http09);
  assert_eq!(hdr_clone.get_path(), "/dummy");
  assert_eq!(hdr_clone.get_method(), &HttpMethod::Get);
  assert_eq!(hdr_clone.get_query().len(), 0);
  Ok(Response::new(StatusCode::OK).with_body(ResponseBody::from_slice("Okay!")))
}

#[test]
pub fn tc1() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "Okay!");
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
}
