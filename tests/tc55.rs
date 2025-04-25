use crate::mock_stream::MockStream;
use tii::{MimeType, ResponseBody, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::ok(
    ResponseBody::from_data_with_gzip_in_memory("Please compress me"),
    MimeType::TextPlain,
  ))
}
#[test]
pub fn tc55() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\nGET /dummy HTTP/1.1\r\nConnection: close\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let got = stream.copy_written_data();
  let expect = [
    72, 84, 84, 80, 47, 49, 46, 49, 32, 50, 48, 48, 32, 79, 75, 13, 10, 67, 111, 110, 116, 101,
    110, 116, 45, 84, 121, 112, 101, 58, 32, 116, 101, 120, 116, 47, 112, 108, 97, 105, 110, 13,
    10, 67, 111, 110, 110, 101, 99, 116, 105, 111, 110, 58, 32, 75, 101, 101, 112, 45, 65, 108,
    105, 118, 101, 13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 76, 101, 110, 103, 116, 104, 58,
    32, 52, 54, 13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 69, 110, 99, 111, 100, 105, 110, 103,
    58, 32, 103, 122, 105, 112, 13, 10, 13, 10, 31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 5, 192, 65, 9, 0,
    0, 8, 3, 192, 42, 139, 100, 5, 145, 253, 28, 138, 235, 15, 94, 52, 211, 68, 141, 246, 104, 67,
    124, 39, 98, 160, 233, 18, 0, 0, 0, 72, 84, 84, 80, 47, 49, 46, 49, 32, 50, 48, 48, 32, 79, 75,
    13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101, 58, 32, 116, 101, 120, 116,
    47, 112, 108, 97, 105, 110, 13, 10, 67, 111, 110, 110, 101, 99, 116, 105, 111, 110, 58, 32, 67,
    108, 111, 115, 101, 13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 76, 101, 110, 103, 116, 104,
    58, 32, 52, 54, 13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 69, 110, 99, 111, 100, 105, 110,
    103, 58, 32, 103, 122, 105, 112, 13, 10, 13, 10, 31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 5, 192, 65,
    9, 0, 0, 8, 3, 192, 42, 139, 100, 5, 145, 253, 28, 138, 235, 15, 94, 52, 211, 68, 141, 246,
    104, 67, 124, 39, 98, 160, 233, 18, 0, 0, 0,
  ];
  assert_eq!(got.as_slice(), expect.as_slice());
}
