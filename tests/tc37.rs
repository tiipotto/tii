use crate::mock_stream::MockStream;
use tii::ServerBuilder;
use tii::RequestContext;
use tii::Response;
use tii::TiiResult;
use tii::{AcceptQualityMimeType, MimeType, QValue};

mod mock_stream;

fn filter(ctx: &mut RequestContext) -> TiiResult<()> {
  if ctx.request_head().get_path() == "/dummy" {
    ctx.request_head_mut().set_accept(vec![AcceptQualityMimeType::from_mime(
      MimeType::TextPlain,
      QValue::default(),
    )]);
  }

  Ok(())
}

fn route(_: &RequestContext) -> Response {
  Response::no_content()
}

#[test]
pub fn tc36() {
  let server = ServerBuilder::default()
    .router(|rt| {
      rt.with_pre_routing_request_filter(filter)?
        .begin_get("/dummy", |r| r.produces(MimeType::TextPlain).endpoint(route))?
        .begin_get("/dummy2", |r| r.produces(MimeType::TextPlain).endpoint(route))
    })
    .expect("ERR")
    .build();

  let stream =
    MockStream::with_str("GET /dummy HTTP/1.1\r\nAccept: text/html\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");

  let stream =
    MockStream::with_str("GET /dummy2 HTTP/1.1\r\nAccept: text/html\r\nHdr: test\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 406 Not Acceptable\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
