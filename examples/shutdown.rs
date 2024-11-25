use humpty::extras::network_utils::unspecified_socket_to_loopback;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;

use humpty::http::mime::MimeType;
use humpty::humpty_error::HumptyResult;
use std::error::Error;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::{io, thread};

fn hello(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
}

fn main() -> Result<(), Box<dyn Error>> {
  let (shutdown_app, app_rx) = mpsc::sync_channel(1);

  let app = HumptyBuilder::builder_arc(|builder| {
    builder
      .router(|router| router.route_any("/*", hello))?
      .with_connection_timeout(Some(Duration::from_secs(5)))?
      .ok()
  })
  .expect("ERROR");

  let listen = TcpListener::bind("0.0.0.0:8080")?;
  let addr = listen.local_addr()?;
  println!("successfully listening on {addr}");

  // Send shutdown signal after 5 seconds, well after threads have started working
  let t = spawn(move || {
    sleep(Duration::from_secs(5));
    shutdown_app.send(()).unwrap();
    // wake up the TcpListener loop
    TcpStream::connect(unspecified_socket_to_loopback(addr).unwrap()).unwrap();
  });

  for stream in listen.incoming() {
    if app_rx.try_recv().is_ok() {
      println!("shutdown receieved. breaking out of loop");
      break;
    }
    let app = app.clone();
    thread::spawn(move || {
      app.handle_connection(stream?).expect("ERORR");
      Ok::<(), io::Error>(())
    });
  }

  // The TcpListener can be dropped. Within 5 seconds, it should always be free to be reused.
  drop(listen);
  let _listen = TcpListener::bind("0.0.0.0:8080")?;

  // Wait for thread to fully finish. Unneeded but placed here for full memory tests.
  t.join().unwrap();
  Ok(())
}

#[test]
fn run() {
  main();
}
