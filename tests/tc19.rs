use crate::mock_stream::MockStream;
use tii::RequestContext;
use tii::ServerBuilder;
use tii::TiiResult;
use tii::{Response, StatusCode};

mod mock_stream;

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  let body = ctx.request_body().unwrap();
  let mut data = vec![];
  body.read_to_end(&mut data)?;
  Response::new(StatusCode::OK).with_body(data).into()
}

#[test]
pub fn tc19() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 200 OK\r\nConnection: Close\r\nContent-Length: 21\r\n\r\n123451234567890123456"
  );
}
