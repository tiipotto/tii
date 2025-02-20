use crate::mock_stream::MockStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use tii::RequestContext;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiResult;

mod mock_stream;

static COUNT: AtomicUsize = AtomicUsize::new(0);
fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  COUNT.fetch_add(1, Ordering::SeqCst);
  Ok(Response::no_content())
}
#[test]
pub fn tc41() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Length: 0\r\n\r\nGET /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Length: 0\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  assert_eq!(COUNT.load(Ordering::SeqCst), 2);

  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\nHTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\n"
  );
}
