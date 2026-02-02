//! Static content server example.
//!
//! ## Important
//! This example must be run from the `static-content` directory to successfully find the paths.
//! This is because content is found relative to the CWD instead of the binary.

use tii::extras::{Connector, TcpConnector, builtin_endpoints};

use tii::ServerBuilder;
use tii::TiiResult;

fn main() -> TiiResult<()> {
  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder.router(|router| {
      router
        .route_any("/", builtin_endpoints::serve_file("./examples/static/pages/index.html"))?
        // Serve the "/img/*" route with files stored in the "./static/images" directory.
        .route_any("/img/*", builtin_endpoints::serve_dir("./examples/static/images"))?
        // Serve a regular file path in the current directory.
        // This means simply appending the request URI to the directory path and looking for a file there.
        .route_any("/examples/*", builtin_endpoints::serve_as_file_path("."))?
        // Redirect requests to "/ferris" to "/img/ferris.png"
        .route_any("/ferris", builtin_endpoints::redirect("/img/ferris.png"))
    })
  })?;

  let _ = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?.join(None);

  Ok(())
}
