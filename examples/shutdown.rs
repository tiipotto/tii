use std::error::Error;
use std::net::TcpListener;
use std::thread::{self, sleep};
use std::time::Duration;

use humpty::extras::tcp_app;
use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;

fn hello(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
}

fn main() -> Result<(), Box<dyn Error>> {
  let humpty_server = HumptyBuilder::builder_arc(|builder| {
    builder
      .router(|router| router.route_any("/*", hello))?
      .with_connection_timeout(Some(Duration::from_secs(5)))?
      .ok()
  })
  .expect("ERROR");

  let mut app = tcp_app::App::new("0.0.0.0:8080", humpty_server)?;
  let done_signal = app.done_receiver().unwrap();
  let t = thread::spawn(move || {
    println!("blocking until app is done");
    done_signal.recv().unwrap();
    println!("app is done executing");
  });

  // Send shutdown signal after 5 seconds, well after threads have started working
  sleep(Duration::from_secs(5));
  app.shutdown().unwrap();

  // With the app having finished shutdown(), the socket can be rebound immediately.
  let _listen = TcpListener::bind("0.0.0.0:8080")?;

  t.join().unwrap();
  Ok(())
}

#[test]
fn run() {
  main();
}
