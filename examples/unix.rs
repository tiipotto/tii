use tii::TiiMimeType;
use tii::TiiRequestContext;
use tii::TiiResponse;

#[cfg(not(unix))]
pub fn main() {
  println!("This program is only intended to run on Unix systems!");
}

#[cfg(unix)]
pub fn main() {
  unix::work().expect("Error");
}
pub fn handle(ctx: &TiiRequestContext) -> TiiResponse {
  if ctx.peer_address() == "unix" {
    //Hello unix->/tmp/tii.sock with GET /path HTTP/1.1
    TiiResponse::ok(
      format!(
        "Hello unix->{} with {}\n",
        ctx.local_address(),
        ctx.request_head().get_raw_status_line()
      ),
      TiiMimeType::TextPlain,
    )
  } else {
    //Hello tcp 127.0.0.1:37548->127.0.0.1:8080 with GET /some/path HTTP/1.1
    TiiResponse::ok(
      format!(
        "Hello tcp {}->{} with {}\n",
        ctx.peer_address(),
        ctx.local_address(),
        ctx.request_head().get_raw_status_line()
      ),
      TiiMimeType::TextPlain,
    )
  }
}

#[cfg(unix)]
mod unix {
  use crate::handle;
  use tii::extras;
  use tii::extras::TiiConnector;
  use tii::TiiBuilder;
  use tii::TiiResult;

  pub fn work() -> TiiResult<()> {
    colog::default_builder().filter_level(log::LevelFilter::Trace).init();

    let tii_server =
      TiiBuilder::builder_arc(|builder| builder.router(|router| router.route_any("/*", handle)))?;

    if std::fs::exists("/tmp/tii.sock")? {
      std::fs::remove_file("/tmp/tii.sock")?;
    }

    //HANDLE TCP CONNECTIONS
    //curl -X GET http://127.0.0.1:8080/some/path
    let tcp = extras::TiiTcpConnector::start_unpooled("0.0.0.0:8080", tii_server.clone())?;

    //HANDLE UNIX CONNECTIONS
    //curl -X GET --unix-socket /tmp/tii.sock http://unix/some/path
    let unix = extras::TiiUnixConnector::start_unpooled("/tmp/tii.sock", tii_server.clone())?;

    //Both of this will block forever
    unix.join(None);
    tcp.join(None);

    Ok(())
  }
}
