use log::LevelFilter;
use std::time::Duration;
use tii::extras::{Connector, TcpConnector};
use tii::{MimeType, RequestContext, Response, ResponseBody, ServerBuilder};

fn in_memory(request: &RequestContext) -> Response {
  let response_body = format!("Path: {} Hello, World!", request.request_head().get_path());
  Response::ok(
    ResponseBody::from_data_with_gzip_in_memory(response_body.as_bytes()),
    MimeType::TextPlain,
  )
}

fn chunked(request: &RequestContext) -> Response {
  let response_body = format!("Path: {} Hello, World!", request.request_head().get_path());
  Response::ok(
    ResponseBody::chunked_gzip(move |sink| {
      sink.write_all(response_body.as_bytes())?;
      Ok(())
    }),
    MimeType::TextPlain,
  )
}

fn main() {
  trivial_log::init_stdout(LevelFilter::Trace).unwrap();
  //curl --compressed -v http://localhost:8080/in_memory/bla
  //curl --compressed -v http://localhost:8080/chunked/bla
  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder
      .router(|router| {
        router.route_get("/in_memory/*", in_memory)?.route_get("/chunked/*", chunked)?.ok()
      })?
      .with_keep_alive_timeout(Some(Duration::ZERO)) //We disable http keep alive.
  })
  .unwrap();

  let _ = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server).unwrap().join(None);
  trivial_log::free()
}
