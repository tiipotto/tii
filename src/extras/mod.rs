pub mod builtin_endpoints;

mod connector;

pub use connector::Connector;
pub(crate) use connector::CONNECTOR_SHUTDOWN_TIMEOUT;

#[cfg(unix)]
mod unix_connector;

#[cfg(unix)]
pub use unix_connector::*;

mod tcp_connector;

pub use tcp_connector::*;

#[cfg(feature = "socket2")]
mod socket2_tcp_connector;

#[cfg(feature = "socket2")]
pub use socket2_tcp_connector::*;
