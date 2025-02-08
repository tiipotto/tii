use crate::mock_stream::MockStream;
use std::io;
use tii::TiiBuilder;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::TiiResult;

mod mock_stream;

fn dummy_route(_ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  unreachable!();
}

#[test]
pub fn tc8() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let mut combined = Vec::new();
  for b in "GET /dummy HTTP/1.1\r\nHdr: ".as_bytes() {
    combined.push(*b);
  }

  combined.push(0xf0);
  combined.push(0x28);
  combined.push(0x8c);
  combined.push(0xbc);

  for b in "test\r\n\r\n\r\n".as_bytes() {
    combined.push(*b);
  }

  let stream = MockStream::with_slice(combined.as_slice());
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  assert_eq!(err.to_string(), "HeaderLineIsNotUsAscii");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
