use crate::mock_stream::MockStream;
use tii::{RequestContext, Response, ServerBuilder};
use tii::{RequestHeadParsingError, TiiError, TiiResult};

mod mock_stream;
fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}
#[test]
pub fn tc48() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?query=bla%1&q=2 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidQueryString(n)) => {
      assert_eq!(n, "query=bla%1&q=2");
    }
    _ => panic!("unexpected error: {:?}", err),
  }

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}

#[test]
pub fn tc48_2() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?query=bla%1 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidQueryString(n)) => {
      assert_eq!(n, "query=bla%1");
    }
    _ => panic!("unexpected error: {:?}", err),
  }

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "");
}
