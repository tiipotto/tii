use crate::mock_stream::MockStream;
use tii::{MimeCharset, MimeType, RequestContext, Response, ServerBuilder, TiiResult};

mod mock_stream;
fn utf8(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::not_found_no_body())
}

fn ascii(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::no_content())
}

#[test]
pub fn tc64() {
  let server = ServerBuilder::default()
    .router(|rt| {
      rt.put("/*")
        .consumes((MimeType::TextPlain, MimeCharset::UsAscii))
        .endpoint(ascii)?
        .put("/*")
        .consumes((MimeType::TextPlain, MimeCharset::Utf8))
        .endpoint(utf8)
    })
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: 1\r\n\r\nA");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 404 Not Found\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n");

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Type: text/plain; charset=us-ascii\r\nContent-Length: 1\r\n\r\nA");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n"
  );

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Type: text/pain; charset=us-ascii\r\nContent-Length: 1\r\n\r\nA");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 415 Unsupported Media Type\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n"
  );
}
