//! Tii is a very fast, robust and flexible HTTP/1.1 web server crate which allows you to develop web applications in Rust. With no dependencies, it is very quick to compile and produces very small binaries, as well as being very resource-efficient.

#![warn(missing_docs)]

pub mod http;
pub mod websocket;

mod default_functions;
mod functional_traits;
pub mod tii_builder;
pub mod tii_error;
pub use tii_error::TiiError;
pub mod stream;
pub mod tii_router;
pub mod tii_router_builder;
pub mod tii_server;
#[cfg(feature = "tls")]
mod tls_stream;
mod util;

/// Extra utilities that can be useful for many projects but should not be part of tii itself.
/// This stuff might be moved to its own crate at some point.
/// Nothing inside tii can depend on something in extras!
#[cfg(feature = "extras")]
pub mod extras;

#[cfg(feature = "tls")]
pub use tls_stream::{TiiTlsStream, TlsCapableStream};
