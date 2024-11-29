pub mod builtin_endpoints;

/// Convenience functions related to networking
pub mod network_utils;

/// A very simple TCP application that spawns a thread per connection.
/// See examples for usage, in particular `shutdown`.
pub mod tcp_app;

#[cfg(unix)]
mod unix_connector;

#[cfg(unix)]
pub use unix_connector::*;

mod tcp_connector;

pub use tcp_connector::*;
