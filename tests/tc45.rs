use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;

fn rewrite_filter(request: &mut RequestContext) {
  assert_eq!(request.get_query_param("query"), Some("bla"));
  request.set_query(vec![("query".to_string(), "blub".to_string())])
}
fn dummy_route(request: &RequestContext) -> TiiResult<Response> {
  assert_eq!(request.get_query_param("query"), Some("blub"));
  Ok(Response::no_content())
}
#[test]
pub fn tc45() {
  let server = ServerBuilder::default()
    .router(|rt| rt.with_pre_routing_request_filter(rewrite_filter)?.route_get("/*", dummy_route))
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("GET /dummy?query=bla HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
