use crate::mock_stream::MockStream;
use log::LevelFilter;
use tii::TiiResult;
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;
fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  let mut data = Vec::new();
  ctx.request_body().unwrap().read_to_end(&mut data)?;

  assert_eq!(String::from("{ \"mydummy\" : \"json\" }\n"), String::from_utf8(data).unwrap());
  Ok(Response::no_content())
}
#[test]
pub fn tc53() {
  trivial_log::init_stderr(LevelFilter::Trace).unwrap();
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let mut data = Vec::<u8>::new();
  data.extend_from_slice(b"POST /dummy HTTP/1.1\r\n");
  data.extend_from_slice(b"Content-Encoding: gzip\r\n");
  data.extend_from_slice(b"Content-Length: 43\r\n");
  data.extend_from_slice(b"Content-Type: application/json\r\n");
  data.extend_from_slice(b"Connection: keep-alive\r\n");
  data.extend_from_slice(b"\r\n");

  //echo '{ "mydummy" : "json" }' | gzip > tc53.gz
  data.extend_from_slice(include_bytes!("./tc53.gz").as_ref());
  data.extend_from_slice(b"GET /404 HTTP/1.1\r\n\r\n");

  let stream = MockStream::with_slice(data.as_slice());
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\nHTTP/1.1 404 Not Found\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n"
  );
  trivial_log::free();
}

#[test]
pub fn tc53_b() {
  trivial_log::init_stderr(LevelFilter::Trace).unwrap();
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let mut data = Vec::<u8>::new();
  data.extend_from_slice(b"POST /dummy HTTP/1.1\r\n");
  data.extend_from_slice(b"Transfer-Encoding: gzip\r\n");
  data.extend_from_slice(b"Content-Length: 23\r\n");
  data.extend_from_slice(b"Content-Type: application/json\r\n");
  data.extend_from_slice(b"Connection: keep-alive\r\n");
  data.extend_from_slice(b"\r\n");

  //echo '{ "mydummy" : "json" }' | gzip > tc53.gz
  data.extend_from_slice(include_bytes!("./tc53.gz").as_ref());
  data.extend_from_slice(b"GET /404 HTTP/1.1\r\n\r\n"); //Won't be executed!

  let stream = MockStream::with_slice(data.as_slice());
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  //Note: we expect close here! due to Transfer-Encoding: gzip not having a proper impl yet.
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
  trivial_log::free();
}

#[test]
pub fn tc53_c() {
  trivial_log::init_stderr(LevelFilter::Trace).unwrap();
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let mut data = Vec::<u8>::new();
  data.extend_from_slice(b"POST /dummy HTTP/1.1\r\n");
  data.extend_from_slice(b"Transfer-Encoding: chunked\r\n");
  data.extend_from_slice(b"Content-Encoding: gzip\r\n");
  data.extend_from_slice(b"Content-Type: application/json\r\n");
  data.extend_from_slice(b"Connection: keep-alive\r\n");

  data.extend_from_slice(b"\r\n");

  //echo '{ "mydummy" : "json" }' | gzip > tc53.gz
  let sl = include_bytes!("./tc53.gz").as_ref();

  data.extend_from_slice(b"A\r\n");
  data.extend_from_slice(&sl[0..10]);
  data.extend_from_slice(b"\r\n");
  data.extend_from_slice(b"21\r\n");
  data.extend_from_slice(&sl[10..43]);
  data.extend_from_slice(b"\r\n");
  data.extend_from_slice(b"0\r\n\r\n");
  data.extend_from_slice(b"GET /404 HTTP/1.1\r\n\r\n");

  let stream = MockStream::with_slice(data.as_slice());
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  let data = stream.copy_written_data_to_string();
  assert_eq!(
    data,
    "HTTP/1.1 204 No Content\r\nConnection: Keep-Alive\r\nContent-Length: 0\r\n\r\nHTTP/1.1 404 Not Found\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n"
  );
  trivial_log::free();
}
