use crate::mock_stream::MockStream;
use log::LevelFilter;
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
pub fn tc31() {
  trivial_log::init_stdout(LevelFilter::Trace).unwrap();
  let server = ServerBuilder::builder(|builder| {
    builder.router(|rt| rt.route_any("/*", dummy_route))?.with_max_head_buffer_size(512)?.ok()
  })
  .expect("ERROR");

  let stream = MockStream::with_str("GET /%BF HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    TiiError::RequestHeadParsing(RequestHeadParsingError::InvalidPathUrlEncoding(dta)) => {
      assert_eq!(dta.as_str(), "/%BF");
    }
    e => panic!("Unexpected error {e}"),
  }
  trivial_log::free();
}
