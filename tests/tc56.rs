use crate::mock_stream::MockStream;
use tii::{MimeType, ResponseBody, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  let zeroes = vec![0u8; 4096];
  let ones = vec![1u8; 4096];
  let twos = vec![2u8; 4096];

  Ok(Response::ok(
    ResponseBody::chunked_gzip(move |sink| {
      sink.write_all(&zeroes).unwrap();
      sink.write_all(&ones).unwrap();
      sink.write_all(&twos).unwrap();
      Ok(())
    }),
    MimeType::TextPlain,
  ))
}
#[test]
pub fn tc56() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\n\r\nGET /dummy HTTP/1.1\r\nConnection: close\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let got = stream.copy_written_data();
  //NOTE if we change the behavior of how we "chunk" or buffer the data then this value will change, this blob includes the chunked transfer encoding which is impl dependant.
  let expect = [
    72, 84, 84, 80, 47, 49, 46, 49, 32, 50, 48, 48, 32, 79, 75, 13, 10, 67, 111, 110, 116, 101,
    110, 116, 45, 84, 121, 112, 101, 58, 32, 116, 101, 120, 116, 47, 112, 108, 97, 105, 110, 13,
    10, 67, 111, 110, 110, 101, 99, 116, 105, 111, 110, 58, 32, 75, 101, 101, 112, 45, 65, 108,
    105, 118, 101, 13, 10, 84, 114, 97, 110, 115, 102, 101, 114, 45, 69, 110, 99, 111, 100, 105,
    110, 103, 58, 32, 99, 104, 117, 110, 107, 101, 100, 13, 10, 67, 111, 110, 116, 101, 110, 116,
    45, 69, 110, 99, 111, 100, 105, 110, 103, 58, 32, 103, 122, 105, 112, 13, 10, 13, 10, 50, 57,
    13, 10, 31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 237, 192, 1, 13, 0, 0, 8, 192, 32, 181, 127, 104, 123,
    124, 48, 0, 0, 0, 64, 222, 2, 0, 0, 0, 121, 7, 0, 0, 0, 228, 61, 13, 10, 56, 13, 10, 158, 184,
    95, 195, 0, 48, 0, 0, 13, 10, 48, 13, 10, 13, 10, 72, 84, 84, 80, 47, 49, 46, 49, 32, 50, 48,
    48, 32, 79, 75, 13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101, 58, 32, 116,
    101, 120, 116, 47, 112, 108, 97, 105, 110, 13, 10, 67, 111, 110, 110, 101, 99, 116, 105, 111,
    110, 58, 32, 67, 108, 111, 115, 101, 13, 10, 84, 114, 97, 110, 115, 102, 101, 114, 45, 69, 110,
    99, 111, 100, 105, 110, 103, 58, 32, 99, 104, 117, 110, 107, 101, 100, 13, 10, 67, 111, 110,
    116, 101, 110, 116, 45, 69, 110, 99, 111, 100, 105, 110, 103, 58, 32, 103, 122, 105, 112, 13,
    10, 13, 10, 50, 57, 13, 10, 31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 237, 192, 1, 13, 0, 0, 8, 192, 32,
    181, 127, 104, 123, 124, 48, 0, 0, 0, 64, 222, 2, 0, 0, 0, 121, 7, 0, 0, 0, 228, 61, 13, 10,
    56, 13, 10, 158, 184, 95, 195, 0, 48, 0, 0, 13, 10, 48, 13, 10, 13, 10,
  ];
  assert_eq!(got.as_slice(), expect.as_slice());
}
