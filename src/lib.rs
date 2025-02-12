//! Tii is a very fast, robust and flexible HTTP/1.1 web server crate which allows you to develop web applications in Rust. With no dependencies, it is very quick to compile and produces very small binaries, as well as being very resource-efficient.

#![warn(missing_docs)]

mod http;
pub use http::*;

mod websocket;
pub use websocket::*;

mod tii_builder;
pub use tii_builder::*;

mod tii_error;
pub use tii_error::*;
mod stream;
pub use stream::*;
mod tii_router;
pub use tii_router::*;

mod tii_router_builder;
pub use tii_router_builder::*;
mod tii_server;
pub use tii_server::*;
#[cfg(feature = "tls")]
mod tls_stream;
#[cfg(feature = "tls")]
pub use tls_stream::{TlsCapableStream, TlsStream};

/// Extra utilities that can be useful for many projects but should not be part of tii itself.
/// This stuff might be moved to its own crate at some point.
/// Nothing inside tii can depend on something in extras!
#[cfg(feature = "extras")]
pub mod extras;

//Private modules
mod default_functions;
mod functional_traits;

mod util;
