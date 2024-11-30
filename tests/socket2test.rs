use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_error::HumptyResult;

#[allow(dead_code)]
fn hello(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
}
#[cfg(feature = "socket2")]
#[cfg(feature = "extras")]
fn work() -> HumptyResult<()> {
  use humpty::extras::Connector;
  use humpty::humpty_builder::HumptyBuilder;
  use std::io::{Read, Write};
  use std::net::{SocketAddr, TcpStream};
  use std::str::FromStr;
  use std::time::Duration;

  let humpty_server = HumptyBuilder::builder_arc(|builder| {
    builder
      .router(|router| router.route_any("/*", hello))?
      .with_connection_timeout(Some(Duration::from_secs(5)))?
      .ok()
  })?;

  let connector = humpty::extras::Socket2TcpConnector::start("0.0.0.0:18081", humpty_server)?;

  let mut stream =
    TcpStream::connect_timeout(&SocketAddr::from_str("127.0.0.1:18081")?, Duration::from_secs(30))?;
  stream.set_write_timeout(Some(Duration::from_secs(5)))?;
  stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes())?;
  stream.flush()?;
  stream.set_read_timeout(Some(Duration::from_secs(5)))?;
  let mut response = Vec::new();
  stream.read_to_end(&mut response)?;
  assert_eq!(std::str::from_utf8(response.as_slice())?, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: Close\r\nContent-Length: 40\r\n\r\n<html><body><h1>Hello</h1></body></html>");

  connector.shutdown_and_join(None);
  Ok(())
}

#[cfg(feature = "socket2")]
#[cfg(feature = "extras")]
#[test]
pub fn test() {
  work().unwrap();
}
