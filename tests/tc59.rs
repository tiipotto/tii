use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;

struct DummyState {
  state: u32,
}

impl DummyState {
  fn dummy_route(&self, _ctx: &RequestContext) -> TiiResult<Response> {
    assert_eq!(self.state, 5);
    Ok(Response::no_content())
  }
}

#[test]
pub fn tc59() {
  let state = DummyState { state: 5 };

  let my_state: &'static DummyState = Box::leak(Box::new(state));

  let server = ServerBuilder::default()
    .router(|rt| rt.route_get("/*", (my_state, DummyState::dummy_route)))
    .expect("ERR")
    .build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Length: 0\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let got = String::from_utf8(stream.copy_written_data()).unwrap();

  assert_eq!(
    got.as_str(),
    "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n"
  );
}
