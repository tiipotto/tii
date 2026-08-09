use crate::mock_stream::MockStream;
use std::io::Cursor;
use tii::{HttpHeaderName, MimeType, ResponseBody, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(request: &RequestContext) -> TiiResult<Response> {
  let x = request.request_body().unwrap();
  let data = x.read_to_vec()?;
  let sz = data.len() as u64;
  let cursor = Cursor::new(data);
  let range = request.get_byte_range().unwrap();
  let (start, _, len) = range.get_values_for_size(sz).unwrap();
  let resb = ResponseBody::from_file_ranged(cursor, start, len).unwrap();

  Ok(Response::partial_content(resb, MimeType::TextPlain).with_header(
    HttpHeaderName::ContentRange,
    range.get_content_range_header_for_size(sz).unwrap(),
  )?)
}
#[test]
pub fn tc71a() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nRange: bytes=5-\r\nContent-Length: 7\r\n\r\nABCEDFG");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 206 Partial Content\r\nContent-Type: text/plain\r\nContent-Range: bytes 5-6/7\r\nConnection: Keep-Alive\r\nContent-Length: 2\r\n\r\nFG");
}

#[test]
pub fn tc71b() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nRange: bytes=-5\r\nContent-Length: 7\r\n\r\nABCEDFG");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 206 Partial Content\r\nContent-Type: text/plain\r\nContent-Range: bytes 2-6/7\r\nConnection: Keep-Alive\r\nContent-Length: 5\r\n\r\nCEDFG");
}

#[test]
pub fn tc71c() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nRange: bytes=3-5\r\nContent-Length: 7\r\n\r\nABCEDFG");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 206 Partial Content\r\nContent-Type: text/plain\r\nContent-Range: bytes 3-5/7\r\nConnection: Keep-Alive\r\nContent-Length: 3\r\n\r\nEDF");
}
