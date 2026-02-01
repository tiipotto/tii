use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use log::info;
use tii::extras::{self, Connector};
use tii::{MimeType, RequestContext, Response, ServerBuilder, TiiResult};

fn hello(_: &RequestContext) -> TiiResult<Response> {
  Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
}

fn main() -> TiiResult<()> {
  //See https://github.com/rust-lang/rust/issues/135608
  // Workaround for valgrind.
  thread::spawn(actual_main).join().unwrap()
}
fn actual_main() -> TiiResult<()> {
  trivial_log::init_std(log::LevelFilter::Trace).unwrap();

  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder
      .router(|router| router.route_any("/*", hello))?
      .with_connection_timeout(Some(Duration::from_secs(5)))?
      .ok()
  })?;

  let connector = extras::TcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?;

  let mut stream =
    TcpStream::connect_timeout(&SocketAddr::from_str("127.0.0.1:8080")?, Duration::from_secs(30))?;
  stream.set_write_timeout(Some(Duration::from_secs(5)))?;
  stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes())?;
  stream.flush()?;
  stream.set_read_timeout(Some(Duration::from_secs(5)))?;
  let mut response = Vec::new();
  stream.read_to_end(&mut response)?;
  assert_eq!(std::str::from_utf8(response.as_slice())?, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: Close\r\nContent-Length: 40\r\n\r\n<html><body><h1>Hello</h1></body></html>");

  sleep(Duration::from_secs(5));
  assert!(connector.shutdown_and_join(None));

  info!("Shutdown complete");
  drop(connector);

  // With the connector having finished shutdown()
  let _listen = TcpListener::bind("0.0.0.0:8080")?;

  info!("Done");
  trivial_log::free();
  Ok(())
}

#[test]
fn run() {
  main().expect("ERROR");
}
