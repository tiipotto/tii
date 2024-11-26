//! Static content server example.
//!
//! ## Important
//! This example must be run from the `static-content` directory to successfully find the paths.
//! This is because content is found relative to the CWD instead of the binary.

use humpty::extras::builtin_endpoints;
use humpty::extras::tcp_app;

use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyError;
use std::error::Error;
use std::net::TcpListener;
use std::thread;

fn main() -> Result<(), Box<dyn Error>> {
    let humpty_server = HumptyBuilder::builder_arc(|builder| {
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
    })
        .expect("ERROR");

    let _ = tcp_app::App::new("0.0.0.0:8080", humpty_server)?.run();


    Ok(())
}
