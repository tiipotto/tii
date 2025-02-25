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
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let mut data = Vec::<u8>::new();
  data.extend_from_slice(b"POST /mymodule HTTP/1.1\r\n");
  data.extend_from_slice(b"Content-Encoding: gzip\r\n");
  data.extend_from_slice(b"Content-Length: 43\r\n");
  data.extend_from_slice(b"Content-Type: application/json\r\n");
  data.extend_from_slice(b"\r\n");

  //echo '{ "mydummy" : "json" }' | gzip > tc53.gz
  data.extend_from_slice(include_bytes!("./tc53.gz").as_ref());

  let stream = MockStream::with_slice(data.as_slice());
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  trivial_log::free();
}

#[test]
pub fn tc53_b() {
  trivial_log::init_stderr(LevelFilter::Trace).unwrap();
  let server =
      ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let mut data = Vec::<u8>::new();
  data.extend_from_slice(b"POST /mymodule HTTP/1.1\r\n");
  data.extend_from_slice(b"Transfer-Encoding: gzip\r\n");
  data.extend_from_slice(b"Content-Length: 23\r\n");
  data.extend_from_slice(b"Content-Type: application/json\r\n");
  data.extend_from_slice(b"\r\n");

  //echo '{ "mydummy" : "json" }' | gzip > tc53.gz
  data.extend_from_slice(include_bytes!("./tc53.gz").as_ref());

  let stream = MockStream::with_slice(data.as_slice());
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();

  trivial_log::free();
}

