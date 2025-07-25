use crate::mock_stream::MockStream;
use tii::TiiResult;
use tii::{HttpMethod, RequestContext, Response, ServerBuilder};

mod mock_stream;

fn rewrite_meth_filter(request: &mut RequestContext) {
  assert_eq!(request.get_method().clone(), HttpMethod::Get);
  request.set_method(HttpMethod::Put);
}
fn dummy_route_put(request: &RequestContext) -> TiiResult<Response> {
  assert_eq!(request.get_method().clone(), HttpMethod::Put);
  Ok(Response::no_content())
}

fn dummy_route_get(_: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}
#[test]
pub fn tc43() {
  let server = ServerBuilder::default()
    .router(|rt| {
      rt.with_pre_routing_request_filter(rewrite_meth_filter)?
        .route_get("/*", dummy_route_get)?
        .route_put("/*", dummy_route_put)
    })
    .expect("ERR")
    .build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\n\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
