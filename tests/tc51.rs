use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  let q = ctx.get_query();
  assert_eq!(q[0], ("quer•y".to_string(), "bla ".to_string()));
  assert_eq!(q[1], ("q".to_string(), "2 ".to_string()));
  Ok(Response::no_content())
}
#[test]
pub fn tc51() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?quer%E2%80%A2y=bla%20&q=2+ HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
