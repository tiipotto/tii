use crate::mock_stream::MockStream;
use tii::TiiBuilder;
use tii::TiiError;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::{RequestHeadParsingError, TiiResult};

mod mock_stream;

fn dummy_route(_ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  unreachable!()
}

#[test]
pub fn tc30() {
  let server = TiiBuilder::builder(|builder| {
    builder.router(|rt| rt.route_any("/*", dummy_route))?.with_max_head_buffer_size(512)?.ok()
  })
  .expect("ERROR");

  let many_a = String::from_utf8(vec![b'A'; 513]).unwrap();

  let blub = format!("GET / HTTP/1.1\r\nMany: {many_a}\r\n\r\n");

  let stream = MockStream::with_str(blub.as_str());
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::HeaderLineTooLong(dta)) => {
      assert_eq!(dta.len(), 512);
    }
    e => panic!("Unexpected error {e}"),
  }
}
