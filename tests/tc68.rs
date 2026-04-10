use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(request: &RequestContext) -> TiiResult<Response> {
  assert_eq!(request.get_path(), "/dummy+++a");
  Ok(Response::no_content())
}
#[test]
pub fn tc68() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_get("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy+%2b+a HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
