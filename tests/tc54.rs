use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::no_content())
}
#[test]
pub fn tc54() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\nGET /dummy HTTP/1.1\r\nConnection: close\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\nHTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}

#[test]
pub fn tc54_b() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("POST /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\nPOST /dummy HTTP/1.1\r\nConnection: close\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}

#[test]
pub fn tc54_c() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("POST /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Length: 0\r\n\r\nPOST /dummy HTTP/1.1\r\nConnection: close\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\nHTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
