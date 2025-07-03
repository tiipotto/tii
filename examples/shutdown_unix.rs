use std::thread;
use tii::TiiResult;

#[cfg(unix)]
mod unix {
  use std::io::{Read, Write};
  use std::os::unix::net::UnixStream;
  use std::thread::sleep;
  use std::time::Duration;

  use tii::extras::{Connector, UnixConnector};
  use tii::{MimeType, RequestContext, Response, ServerBuilder, TiiResult};

  fn hello(_: &RequestContext) -> TiiResult<Response> {
    Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
  }

  pub fn unix_main() -> TiiResult<()> {
    trivial_log::init_std(log::LevelFilter::Trace).unwrap();

    let tii_server = ServerBuilder::builder_arc(|builder| {
      builder
        .router(|router| router.route_any("/*", hello))?
        .with_read_timeout(Some(Duration::from_secs(5)))?
        .ok()
    })?;

    let connector = UnixConnector::start_unpooled("/tmp/tii.sock", tii_server)?;

    let mut stream = UnixStream::connect("/tmp/tii.sock")?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes())?;
    stream.flush()?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    assert_eq!(
      std::str::from_utf8(response.as_slice())?,
      "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: Close\r\nContent-Length: 40\r\n\r\n<html><body><h1>Hello</h1></body></html>"
    );

    sleep(Duration::from_secs(5));
    connector.shutdown_and_join(None);
    trivial_log::free();
    Ok(())
  }
}

fn main() -> TiiResult<()> {
  //See https://github.com/rust-lang/rust/issues/135608
  // Workaround for valgrind.
  thread::spawn(actual_main).join().unwrap()
}

#[cfg(unix)]
fn actual_main() -> TiiResult<()> {
  unix::unix_main()
}

#[cfg(not(unix))]
pub fn actual_main() -> TiiResult<()> {
  println!("This program is only intended to run on Unix systems!");
  Ok(())
}

#[test]
fn run() {
  main().expect("ERROR");
}
