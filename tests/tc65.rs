use crate::mock_stream::MockStream;
use tii::{MimeCharset, MimeType, RequestContext, Response, ServerBuilder, TiiResult};

mod mock_stream;
fn dummy(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::not_found_no_body())
}
#[test]
pub fn tc65() {
  let server = ServerBuilder::default()
    .router(|rt| rt.put("/*").produces((MimeType::TextPlain, MimeCharset::Utf8)).endpoint(dummy))
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nAccept: text/plain; charset=utf-8\r\nContent-Length: 0\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 404 Not Found\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n");

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nAccept: text/plain; charset=us-ascii\r\nContent-Length: 0\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 406 Not Acceptable\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n"
  );
}
