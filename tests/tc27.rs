use crate::mock_stream::MockStream;
use tii::TiiBuilder;
use tii::TiiHttpVersion;
use tii::TiiRequestContext;
use tii::TiiResponseBody;
use tii::TiiResult;
use tii::{TiiResponse, TiiStatusCode};

mod mock_stream;

fn add_header_filter(request: &mut TiiRequestContext) -> TiiResult<()> {
  request.request_head_mut().set_header("custom-header-name", "custom-header-value")
}

fn dummy_route(ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  assert_eq!(TiiHttpVersion::Http11, ctx.request_head().get_version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));
  assert_eq!(ctx.request_head().get_header("custom-header-name"), Some("custom-header-value"));

  Ok(TiiResponse::new(TiiStatusCode::OK).with_body(TiiResponseBody::from_slice("Okay!")))
}

#[test]
pub fn tc27() {
  let server = TiiBuilder::default()
    .router(|rt| rt.route_any("/dummy", dummy_route)?.with_request_filter(add_header_filter))
    .expect("ERROR")
    .build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");
}
