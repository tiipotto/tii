use crate::mock_stream::MockStream;
use tii::http::request::HttpVersion;
use tii::http::request_context::RequestContext;
use tii::http::response_body::ResponseBody;
use tii::http::{Response, StatusCode};
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;

mod mock_stream;

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  assert_eq!(HttpVersion::Http11, ctx.request_head().version());
  assert_eq!(ctx.get_path_param("param1"), Some("p1"));
  assert_eq!(ctx.get_path_param("param2"), Some("p2"));
  let regex1 = ctx.get_path_param("regex1").unwrap();
  if regex1 != "1234" && regex1 != "0" {
    panic!("{}", regex1)
  }
  assert_eq!(ctx.get_path_param("regex2"), Some("hello/world"));

  Ok(Response::new(StatusCode::OK).with_body(ResponseBody::from(format!("Okay! {}", regex1))))
}

#[test]
pub fn tc28() {
  let server = TiiBuilder::default()
    .router(|rt| {
      rt.route_any("/dummy/{param1}/{param2}/{regex1:^([1-9][0-9]*)|0$}/{regex2:.*}", dummy_route)
    })
    .expect("ERROR")
    .build();

  let stream = MockStream::with_str("GET /dummy/p1/p2/1234/hello/world HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 10\r\n\r\nOkay! 1234");

  let stream = MockStream::with_str("GET /dummy/p1/p2/abc/hello/world HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 404 Not Found\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");

  let stream = MockStream::with_str("GET /dummy/p1/p2/01234/hello/world HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 404 Not Found\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");

  let stream = MockStream::with_str("GET /dummy/p1/p2/0/hello/world HTTP/1.1\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 7\r\n\r\nOkay! 0");
}
