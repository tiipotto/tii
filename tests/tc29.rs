use crate::mock_stream::MockStream;
use tii::RequestContext;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiError;
use tii::{RequestHeadParsingError, TiiResult};

mod mock_stream;

fn dummy_route(_ctx: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}

#[test]
pub fn tc29() {
  let server = ServerBuilder::builder(|builder| {
    builder.router(|rt| rt.route_any("/*", dummy_route))?.with_max_head_buffer_size(512)?.ok()
  })
  .expect("ERROR");

  let many_a = String::from_utf8(vec![b'A'; 513]).unwrap();

  let blub = format!("GET /{many_a} HTTP/1.1\r\n\r\n");

  let stream = MockStream::with_str(blub.as_str());
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::StatusLineTooLong(dta)) => {
      assert_eq!(dta.len(), 512);
    }
    e => panic!("Unexpected error {e}"),
  }
}
