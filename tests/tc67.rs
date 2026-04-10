use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(request: &RequestContext) -> TiiResult<Response> {
  assert_eq!(request.get_query_param("query"), Some("12345"));
  let g = request.parse_query_param::<u64, _>("query").expect("parsing_error").expect("present");
  assert_eq!(g, 12345);
  let g = request.parse_query_param_or::<u64, _>("query", 3456).expect("parsing_error");
  assert_eq!(g, 12345);
  let g = request.parse_query_param_or::<u64, _>("jerry", 3456).expect("parsing_error");
  assert_eq!(g, 3456);
  let g = request.parse_query_param_or_else::<u64, _>("jerry", || 3456).expect("parsing_error");
  assert_eq!(g, 3456);
  let g =
    request.parse_query_param_or_else::<u64, _>("query", || unreachable!()).expect("parsing_error");
  assert_eq!(g, 12345);

  Ok(Response::no_content())
}
#[test]
pub fn tc67() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_get("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy?query=12345 HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
