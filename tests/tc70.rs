use crate::mock_stream::MockStream;
use tii::{MimeType, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(request: &RequestContext) -> TiiResult<Response> {
  let x = request.request_body().unwrap();
  let data = x.read_to_vec()?;
  assert_eq!(data.as_slice(), &[b'A', b'B']);
  Ok(Response::ok(vec![b'A'; 4], MimeType::TextPlain))
}
#[test]
pub fn tc70a() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Length: 2\r\nExpect: 100-continue\r\n\r\nAB");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: 4\r\n\r\nAAAA");
}

#[test]
pub fn tc70b() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str(
    "PUT /dummy HTTP/1.0\r\nContent-Length: 2\r\nExpect: 100-continue\r\n\r\nAB",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.0 100 Continue\r\n\r\nHTTP/1.0 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\r\nAAAA");
}
