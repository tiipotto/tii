use log::LevelFilter;
use std::net::TcpListener;
use std::time::Duration;
use tii::MimeType;
use tii::RequestContext;
use tii::Response;
use tii::ServerBuilder;

fn hello_world(request: &RequestContext) -> Response {
  let response_body = format!("Path: {} Hello, World!", request.get_path());
  Response::ok(response_body, MimeType::TextPlain)
}
fn main() {
  trivial_log::init_stdout(LevelFilter::Info).unwrap();

  let tii_server = ServerBuilder::builder(|builder| {
    builder
      .router(|router| router.route_get("/*", hello_world))?
      .with_keep_alive_timeout(Some(Duration::ZERO)) //We disable http keep alive.
  })
  .unwrap();

  // This does not spawn any threads, everything will be done in the main thread!
  // Connections will be processed 1 at a time.
  let tcp_listen = TcpListener::bind("0.0.0.0:8080").unwrap();
  for tcp_stream in tcp_listen.incoming() {
    if let Err(err) = tii_server.handle_connection(tcp_stream.unwrap()) {
      eprintln!("Error handling request: {err}");
    }
  }
}
