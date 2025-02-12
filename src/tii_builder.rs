//! Provides the core Tii app functionality.

use crate::Response;

use std::sync::Arc;
use std::time::Duration;

/// Represents the Tii app.
pub struct ServerBuilder {
  routers: Vec<Box<dyn Router>>,
  error_handler: ErrorHandler,
  not_found_handler: NotFoundHandler,
  max_head_buffer_size: usize,
  connection_timeout: Option<Duration>,
  read_timeout: Option<Duration>,
  keep_alive_timeout: Option<Duration>,
  request_body_io_timeout: Option<Duration>,
  write_timeout: Option<Duration>,
}

use crate::default_functions::{default_error_handler, default_fallback_not_found_handler};
pub use crate::functional_traits::*;
use crate::tii_error::{TiiError, TiiResult, UserError};
use crate::tii_router::Routeable;
use crate::tii_router_builder::RouterBuilder;
use crate::tii_server::Server;
use crate::RequestContext;

/// Represents a function able to handle an error.
/// The first parameter of type `Option<Request>` will be `Some` if the request could be parsed.
/// Otherwise, it will be `None` and the status code will be `StatusCode::BadRequest`.
///
/// Every app has a default error handler, which simply displays the status code.
/// The source code for this default error handler is copied below since it is a good example.
///
pub type ErrorHandler = fn(&mut RequestContext, TiiError) -> TiiResult<Response>;

/// Handler for request that couldn't route for some reason.
pub type NotRouteableHandler =
  fn(&mut RequestContext, &[Routeable]) -> TiiResult<Response>;

/// Fallback handler if no router handled the request.
pub type NotFoundHandler = fn(&mut RequestContext) -> TiiResult<Response>;

impl Default for ServerBuilder {
  /// Initialises a new Tii app.
  fn default() -> Self {
    Self {
      routers: Vec::new(),
      error_handler: default_error_handler,
      not_found_handler: default_fallback_not_found_handler,
      connection_timeout: None,
      max_head_buffer_size: 8192,
      keep_alive_timeout: None,
      read_timeout: None,
      request_body_io_timeout: None,
      write_timeout: None,
    }
  }
}

impl ServerBuilder {
  /// Build TiiServer using a closure or fn which receives the builder
  pub fn builder<T: FnOnce(ServerBuilder) -> TiiResult<ServerBuilder>>(
    closure: T,
  ) -> TiiResult<Server> {
    closure(ServerBuilder::default()).map(|builder| builder.build())
  }

  /// Build `Arc<TiiServer>` using a closure or fn which receives the builder
  pub fn builder_arc<T: FnOnce(ServerBuilder) -> TiiResult<ServerBuilder>>(
    closure: T,
  ) -> TiiResult<Arc<Server>> {
    closure(ServerBuilder::default()).map(|builder| builder.build_arc())
  }

  /// This method creates the HttpServer from the builder.
  pub fn build(self) -> Server {
    Server::new(
      self.routers,
      self.error_handler,
      self.not_found_handler,
      self.max_head_buffer_size,
      self.connection_timeout,
      self.read_timeout,
      self.keep_alive_timeout,
      self.request_body_io_timeout,
      self.write_timeout,
    )
  }

  /// This method is equivalent to calling `Arc::new(builder.build())`
  pub fn build_arc(self) -> Arc<Server> {
    Arc::new(self.build())
  }

  /// Adds a new host sub-app to the server.
  /// The host can contain wildcards, for example `*.example.com`.
  ///
  /// ## Panics
  /// This function will panic if the host is equal to `*`, since this is the default host.
  /// If you want to add a route to every host, simply add it directly to the main app.
  pub fn add_router(mut self, handler: impl Router + 'static) -> Self {
    self.routers.push(Box::new(handler));
    self
  }

  /// Adds a new router to the server and calls the closure with the new router so it can be configured.
  pub fn router<T: FnOnce(RouterBuilder) -> TiiResult<RouterBuilder>>(
    self,
    builder: T,
  ) -> TiiResult<Self> {
    Ok(self.add_router(builder(RouterBuilder::default())?.build()))
  }

  /// Sets the error handler for the server.
  pub fn with_error_handler(mut self, handler: ErrorHandler) -> TiiResult<Self> {
    self.error_handler = handler;
    Ok(self)
  }

  /// Sets the not found handler for the server.
  pub fn with_not_found_handler(mut self, handler: NotFoundHandler) -> TiiResult<Self> {
    self.not_found_handler = handler;
    Ok(self)
  }

  /// Sets the maximum head buffer size. Default value is 8192.
  ///
  /// This affects the maximum permitted length of a header name + value pair as well
  /// as the maximum length of the status line and therefore the url.
  ///
  /// This value includes protocol overhead such as the ": " separator between header name/value pairs
  /// as well as the HTTP Method and protocol version and the CRLF trailer of each line.
  ///
  /// Setting this value to below a minimum of 0x100/256 is prevented and will cause this fn to return Err.
  ///
  pub fn with_max_head_buffer_size(mut self, size: usize) -> TiiResult<Self> {
    if size < 0x100 {
      return Err(UserError::RequestHeadBufferTooSmall(size).into());
    }
    self.max_head_buffer_size = size;
    Ok(self)
  }

  /// Sets the connection timeout,
  /// the amount of time before tii will close the connection if it sends no data to tii.
  /// If this value is not set then Tii will use the read_timeout for this purpose
  pub fn with_connection_timeout(mut self, timeout: Option<Duration>) -> TiiResult<Self> {
    self.connection_timeout = timeout;
    Ok(self)
  }

  /// Sets the read timeout
  /// the amount of time before tii will time out a connection when reading data at any point.
  /// Different timeouts might overwrite this value for certain aspects.
  /// Default is None = Infinite timeout.
  pub fn with_read_timeout(mut self, timeout: Option<Duration>) -> TiiResult<Self> {
    self.read_timeout = timeout;
    Ok(self)
  }

  /// Sets the write timeout
  /// the amount of time before tii will time out a connection when writing data to the underlying connection at any point.
  /// Default is None = Infinite timeout.
  pub fn with_write_timeout(mut self, timeout: Option<Duration>) -> TiiResult<Self> {
    self.write_timeout = timeout;
    Ok(self)
  }

  /// Sets the keep alive timeout.
  /// Setting this to None (The Default) will make tii use the read timeout instead.
  /// Setting this value to 0 will disable keep-alives and cause Tii to signal to the client
  /// that keep-alives are not supported by setting "Connection: Close" on every HTTP1/1 response.
  ///
  /// Otherwise, tii will wait this amount of time for the client to send at least 1 byte of the next request.
  pub fn with_keep_alive_timeout(mut self, timeout: Option<Duration>) -> TiiResult<Self> {
    self.keep_alive_timeout = timeout;
    Ok(self)
  }

  /// Sets the amount of time tii will wait for the client to produce at least a single byte of a request
  /// body before returning the `TimedOut` error.
  /// A value of None will cause the read timeout to be used.
  pub fn with_request_body_timeout(mut self, timeout: Option<Duration>) -> TiiResult<Self> {
    self.request_body_io_timeout = timeout;
    Ok(self)
  }

  /// Helper fn to make builder code look a bit cleaner
  pub fn ok(self) -> TiiResult<Self> {
    Ok(self)
  }
}
