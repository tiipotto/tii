use crate::mock_stream::MockStream;
use tii::HttpVersion;
use tii::RequestContext;
use tii::ResponseBody;
use tii::ServerBuilder;
use tii::TiiResult;
use tii::{Response, StatusCode};

mod mock_stream;

fn add_header_filter(request: &mut RequestContext) -> TiiResult<()> {
  request.request_head_mut().set_header("custom-header-name", "custom-header-value")
}

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  assert_eq!(HttpVersion::Http11, ctx.request_head().get_version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));
  assert_eq!(ctx.request_head().get_header("custom-header-name"), Some("custom-header-value"));

  Ok(Response::new(StatusCode::OK).with_body(ResponseBody::from_slice("Okay!")))
}

#[test]
pub fn tc27() {
  let server = ServerBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route)?.with_request_filter(add_header_filter))
    .expect("ERROR")
    .build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");
}
