use crate::mock_stream::MockStream;
use std::sync::atomic::AtomicUsize;
use tii::TiiBuilder;
use tii::TiiHttpMethod;
use tii::TiiHttpVersion;
use tii::TiiRequestContext;
use tii::TiiResponseBody;
use tii::TiiResult;
use tii::{TiiResponse, TiiStatusCode};

mod mock_stream;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn dummy_route(ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  assert_eq!(TiiHttpVersion::Http11, ctx.request_head().get_version());
  assert_eq!(ctx.request_head().get_method().as_str(), "QUERY");

  let mut buf = Vec::new();
  let rt = ctx.request_body().unwrap().read_to_end(&mut buf).unwrap();
  assert_eq!(rt, 5);
  assert_eq!(String::from_utf8_lossy(&buf), "12345");

  Ok(TiiResponse::new(TiiStatusCode::OK).with_body(TiiResponseBody::from_slice("Okay!")))
}

#[test]
pub fn tc18() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_method(TiiHttpMethod::from("QUERY"), "/dummy", dummy_route))
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("QUERY /dummy HTTP/1.1\r\nContent-Length: 5\r\n\r\n12345");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");
  assert_eq!(COUNTER.load(std::sync::atomic::Ordering::SeqCst), 1);
}
