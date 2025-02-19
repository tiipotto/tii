use crate::mock_stream::MockStream;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiResult;
use tii::{RequestContext, RequestHeadParsingError, TiiError};

mod mock_stream;

fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}
#[test]
pub fn tc40() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy&query=4 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidPath(n)) => {
      assert_eq!(n, "/dummy&query=4");
    }
    _ => panic!("unexpected error: {:?}", err),
  }

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
