use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;

#[cfg(not(unix))]
pub fn main() {
  println!("This program is only intended to run on Unix systems!");
}

#[cfg(unix)]
pub fn main() {
  unix::work().expect("Error");
}
pub fn handle(ctx: &RequestContext) -> Response {
  if ctx.peer_address() == "unix" {
    //Hello unix->/tmp/humpty.sock with GET /path HTTP/1.1
    Response::ok(
      format!(
        "Hello unix->{} with {}\n",
        ctx.local_address(),
        ctx.request_head().raw_status_line()
      ),
      MimeType::TextPlain,
    )
  } else {
    //Hello tcp 127.0.0.1:37548->127.0.0.1:8080 with GET /some/path HTTP/1.1
    Response::ok(
      format!(
        "Hello tcp {}->{} with {}\n",
        ctx.peer_address(),
        ctx.local_address(),
        ctx.request_head().raw_status_line()
      ),
      MimeType::TextPlain,
    )
  }
}

#[cfg(unix)]
mod unix {
  use crate::handle;
  use humpty::extras;
  use humpty::humpty_builder::HumptyBuilder;
  use humpty::humpty_error::HumptyResult;

  pub fn work() -> HumptyResult<()> {
    colog::default_builder().filter_level(log::LevelFilter::Trace).init();

    let humpty_server = HumptyBuilder::builder_arc(|builder| {
      builder.router(|router| router.route_any("/*", handle))
    })?;

    if std::fs::exists("/tmp/humpty.sock")? {
      std::fs::remove_file("/tmp/humpty.sock")?;
    }

    //HANDLE TCP CONNECTIONS
    //curl -X GET http://127.0.0.1:8080/some/path
    let tcp = extras::TcpConnector::new("0.0.0.0:8080", humpty_server.clone())?;

    //HANDLE UNIX CONNECTIONS
    //curl -X GET --unix-socket /tmp/humpty.sock http://unix/some/path
    let unix = extras::UnixConnector::new("/tmp/humpty.sock", humpty_server.clone())?;

    //Both of this will block forever
    unix.join();
    tcp.join();

    Ok(())
  }
}
