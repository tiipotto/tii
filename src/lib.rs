//! Humpty is a very fast, robust and flexible HTTP/1.1 web server crate which allows you to develop web applications in Rust. With no dependencies, it is very quick to compile and produces very small binaries, as well as being very resource-efficient.

#![warn(missing_docs)]

pub mod handlers;
pub mod http;
pub mod websocket;

mod default_functions;
mod functional_traits;
pub mod humpty_builder;
pub mod humpty_error;
pub mod humpty_router;
pub mod humpty_router_builder;
pub mod humpty_server;
mod krauss;
mod percent;
pub mod stream;
mod thread;
#[cfg(feature = "ssl")]
mod tls_stream;
mod util;

#[cfg(feature = "ssl")]
pub use tls_stream::{HumptyTlsStream, TlsCapableStream};
