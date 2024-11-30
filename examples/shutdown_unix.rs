use humpty::humpty_error::HumptyResult;

#[cfg(unix)]
mod unix {
  use colog::format::{CologStyle, DefaultCologStyle};
  use humpty::extras::{Connector, UnixConnector};
  use humpty::http::mime::MimeType;
  use humpty::http::request_context::RequestContext;
  use humpty::http::Response;
  use humpty::humpty_builder::HumptyBuilder;
  use humpty::humpty_error::HumptyResult;
  use std::io::{Read, Write};
  use std::os::unix::net::UnixStream;
  use std::thread::sleep;
  use std::time::Duration;

  fn hello(_: &RequestContext) -> HumptyResult<Response> {
    Ok(Response::ok("<html><body><h1>Hello</h1></body></html>", MimeType::TextHtml))
  }

  pub fn unix_main() -> HumptyResult<()> {
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

    let humpty_server = HumptyBuilder::builder_arc(|builder| {
      builder
        .router(|router| router.route_any("/*", hello))?
        .with_read_timeout(Some(Duration::from_secs(5)))?
        .ok()
    })?;

    let connector = UnixConnector::start_unpooled("/tmp/humpty.sock", humpty_server)?;

    let mut stream = UnixStream::connect("/tmp/humpty.sock")?;
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
fn main() -> HumptyResult<()> {
  unix::unix_main()
}

#[cfg(not(unix))]
pub fn main() -> HumptyResult<()> {
  println!("This program is only intended to run on Unix systems!");
  Ok(())
}

#[test]
fn run() {
  main().expect("ERROR");
}
