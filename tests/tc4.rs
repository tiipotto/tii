use crate::mock_stream::MockStream;
use std::io;
use tii::TiiBuilder;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::TiiResult;

mod mock_stream;

fn dummy_route(_ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  unreachable!()
}

#[test]
pub fn tc4() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("POST /dummy\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "MethodNotSupportedByHttpVersion(Http09, Post)");

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
