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
pub fn tc31() {
  let server = TiiBuilder::builder(|builder| {
    builder.router(|rt| rt.route_any("/*", dummy_route))?.with_max_head_buffer_size(512)?.ok()
  })
  .expect("ERROR");

  let stream = MockStream::with_str("GET /%BF HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::PathInvalidUrlEncoding(dta)) => {
      assert_eq!(dta.as_str(), "/%BF");
    }
    e => panic!("Unexpected error {e}"),
  }
}
