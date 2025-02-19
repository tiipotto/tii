use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;

fn rewrite_filter(request: &mut RequestContext) {
  assert_eq!(request.request_head().get_path(), "/two");
  request.request_head_mut().set_path("/one");
}
fn dummy_route_one(request: &RequestContext) -> TiiResult<Response> {
  assert_eq!(request.request_head().get_path(), "/one");
  Ok(Response::no_content())
}

fn dummy_route_two(_: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}
#[test]
pub fn tc44() {
  let server = ServerBuilder::default()
    .router(|rt| {
      rt.with_pre_routing_request_filter(rewrite_filter)?
        .route_get("/one", dummy_route_one)?
        .route_put("/two", dummy_route_two)
    })
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("GET /two HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
