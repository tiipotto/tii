use crate::mock_stream::MockStream;
use tii::{HttpMethod, MimeType, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(request: &RequestContext) -> TiiResult<Response> {
  if request.get_method() == HttpMethod::Head {
    let mut resp = Response::ok(vec![b'A'; 4], MimeType::TextPlain);
    resp.omit_body = true;
    return Ok(resp);
  }
  Ok(Response::ok(vec![b'A'; 4], MimeType::TextPlain))
}
#[test]
pub fn tc69() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\nHEAD /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: 4\r\n\r\nAAAAHTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 4\r\n\r\n");
}
