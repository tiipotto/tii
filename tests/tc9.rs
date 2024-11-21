use crate::mock_stream::MockStream;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use std::io;

mod mock_stream;

fn dummy_route(_ctx: &RequestContext) -> HumptyResult<Response> {
  unreachable!();
}

#[test]
pub fn tc9() {
  let server = HumptyBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).build();

  let mut combined = Vec::new();
  for b in "GET /dummy".as_bytes() {
    combined.push(*b);
  }

  combined.push(0xf0);
  combined.push(0x28);
  combined.push(0x8c);
  combined.push(0xbc);

  for b in " HTTP/1.1\r\nHdr: test\r\n\r\n".as_bytes() {
    combined.push(*b);
  }

  let stream = MockStream::with_slice(combined.as_slice());
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "StatusLineContainsInvalidBytes");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
