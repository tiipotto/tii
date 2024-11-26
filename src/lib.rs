//! Humpty is a very fast, robust and flexible HTTP/1.1 web server crate which allows you to develop web applications in Rust. With no dependencies, it is very quick to compile and produces very small binaries, as well as being very resource-efficient.

#![warn(missing_docs)]

pub use humpty_error::HumptyError;

pub mod handlers;
pub mod http;
pub mod humpty_builder;
pub mod humpty_error;
pub mod humpty_router;
pub mod humpty_router_builder;
pub mod humpty_server;
pub mod stream;
pub mod websocket;
#[cfg(feature = "ssl")]
pub use tls_stream::{HumptyTlsStream, TlsCapableStream};

/// Extra utilities that can be useful for many projects but should not be part of humpty itself.
/// This stuff might be moved to its own crate at some point.
/// Nothing inside humpty can depend on something in extras!
pub mod extras;

mod default_functions;
mod functional_traits;
mod krauss;
mod percent;
#[cfg(feature = "ssl")]
mod tls_stream;
mod util;
