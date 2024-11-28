use crate::mock_stream::MockStream;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::{HumptyResult, RequestHeadParsingError};
use humpty::HumptyError;

mod mock_stream;

fn dummy_route(_ctx: &RequestContext) -> HumptyResult<Response> {
  unreachable!()
}

#[test]
pub fn tc31() {
  let server = HumptyBuilder::builder(|builder| {
    builder.router(|rt| rt.route_any("/*", dummy_route))?.with_max_head_buffer_size(512)?.ok()
  })
  .expect("ERROR");

  let stream = MockStream::with_str("GET /%BF HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  let err = server.handle_connection(con).unwrap_err();
  match err {
    HumptyError::RequestHeadParsing(RequestHeadParsingError::PathInvalidUrlEncoding(dta)) => {
      assert_eq!(dta.as_str(), "/%BF");
    }
    e => panic!("Unexpected error {e}"),
  }
}
