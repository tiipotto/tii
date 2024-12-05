use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use std::net::TcpListener;
use std::time::Duration;

fn hello_world(request: &RequestContext) -> Response {
  let response_body = format!("Path: {} Hello, World!", request.request_head().path());
  Response::ok(response_body, MimeType::TextPlain)
}
fn main() {
  let humpty_server = HumptyBuilder::builder(|builder| {
    builder
      .router(|router| router.route_get("/*", hello_world))?
      .with_keep_alive_timeout(Some(Duration::ZERO)) //We disable http keep alive.
  })
  .unwrap();

  // This does not spawn any threads, everything will be done in the main thread!
  // Connections will be processed 1 at a time.
  let tcp_listen = TcpListener::bind("0.0.0.0:8080").unwrap();
  for tcp_stream in tcp_listen.incoming() {
    if let Err(err) = humpty_server.handle_connection(tcp_stream.unwrap()) {
      eprintln!("Error handling request: {}", err);
    }
  }
}
