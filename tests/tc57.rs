use crate::mock_stream::MockStream;
use tii::{MimeType, ResponseBody, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;

static STATIC_DATA: &[u8] = include_bytes!("tc57.txt");

fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::ok(ResponseBody::from_static_slice(STATIC_DATA), MimeType::TextPlain))
}
#[test]
pub fn tc57() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let got = stream.copy_written_data();
  let mut expect = Vec::new();
  expect.extend_from_slice("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: 445\r\n\r\n".as_bytes());
  expect.extend_from_slice(STATIC_DATA);

  assert_eq!(got.as_slice(), expect.as_slice());
}
