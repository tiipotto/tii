use std::net::TcpListener;
use std::time::Duration;
use tii::TiiBuilder;
use tii::TiiMimeType;
use tii::TiiRequestContext;
use tii::TiiResponse;

fn hello_world(request: &TiiRequestContext) -> TiiResponse {
  let response_body = format!("Path: {} Hello, World!", request.request_head().get_path());
  TiiResponse::ok(response_body, TiiMimeType::TextPlain)
}
fn main() {
  let tii_server = TiiBuilder::builder(|builder| {
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
      eprintln!("Error handling request: {}", err);
    }
  }
}
