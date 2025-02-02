use crate::mock_stream::MockStream;
use std::io;
use tii::http::request_context::RequestContext;
use tii::http::Response;
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;

mod mock_stream;

fn dummy_route(_ctx: &RequestContext) -> TiiResult<Response> {
  unreachable!();
}

#[test]
pub fn tc11() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERROR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1 sadness\r\nHdr test\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "StatusLineTooManyWhitespaces");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
