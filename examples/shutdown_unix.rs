use tii::TiiResult;

#[cfg(unix)]
mod unix {
  use colog::format::{CologStyle, DefaultCologStyle};
  use std::io::{Read, Write};
  use std::os::unix::net::UnixStream;
  use std::thread::sleep;
  use std::time::Duration;
  use tii::extras::{Connector, UnixConnector};
  use tii::MimeType;
  use tii::RequestContext;
  use tii::Response;
  use tii::ServerBuilder;
  use tii::TiiResult;

  fn hello(_: &RequestContext) -> TiiResult<Response> {
    Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
  }

  pub fn unix_main() -> TiiResult<()> {
    _ = colog::default_builder()
      .format(|buf, record| {
        let sep = DefaultCologStyle.line_separator();
        let prefix = DefaultCologStyle.prefix_token(&record.level());
        writeln!(
          buf,
          "{} {:?} {}",
          prefix,
          std::thread::current().id(),
          record.args().to_string().replace('\n', &sep),
        )
      })
      .filter_level(log::LevelFilter::Trace)
      .try_init();

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
    assert_eq!(std::str::from_utf8(response.as_slice())?, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: Close\r\nContent-Length: 40\r\n\r\n<html><body><h1>Hello</h1></body></html>");

    sleep(Duration::from_secs(5));
    connector.shutdown_and_join(None);
    Ok(())
  }
}

#[cfg(unix)]
fn main() -> TiiResult<()> {
  unix::unix_main()
}

#[cfg(not(unix))]
pub fn main() -> TiiResult<()> {
  println!("This program is only intended to run on Unix systems!");
  Ok(())
}

#[test]
fn run() {
  main().expect("ERROR");
}
