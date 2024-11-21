//! Provides the core Humpty app functionality.

use crate::http::response::Response;

use std::sync::Arc;
use std::time::Duration;

/// Represents the Humpty app.
pub struct HumptyBuilder {
  routers: Vec<Box<dyn Router>>,
  error_handler: ErrorHandler,
  not_found_handler: NotFoundHandler,
  connection_timeout: Option<Duration>,
}

use crate::default_functions::{default_error_handler, default_fallback_not_found_handler};
pub use crate::functional_traits::*;
use crate::http::request_context::RequestContext;
use crate::humpty_error::{HumptyError, HumptyResult};
use crate::humpty_router::RouteHandler;
use crate::humpty_router_builder::HumptyRouterBuilder;
use crate::humpty_server::HumptyServer;

/// Represents a function able to handle an error.
/// The first parameter of type `Option<Request>` will be `Some` if the request could be parsed.
/// Otherwise, it will be `None` and the status code will be `StatusCode::BadRequest`.
///
/// Every app has a default error handler, which simply displays the status code.
/// The source code for this default error handler is copied below since it is a good example.
///
pub type ErrorHandler = fn(&mut RequestContext, HumptyError) -> HumptyResult<Response>;

/// Handler for request that couldn't route for some reason.
pub type NotRouteableHandler = fn(&mut RequestContext, &[RouteHandler]) -> HumptyResult<Response>;

/// Fallback handler if no router handled the request.
pub type NotFoundHandler = fn(&mut RequestContext) -> HumptyResult<Response>;

impl Default for HumptyBuilder {
  /// Initialises a new Humpty app.
  fn default() -> Self {
    Self {
      routers: Vec::new(),
      error_handler: default_error_handler,
      not_found_handler: default_fallback_not_found_handler,
      connection_timeout: None,
    }
  }
}

impl HumptyBuilder {
  /// This method creates the HttpServer from the builder.
  pub fn build(self) -> HumptyServer {
    HumptyServer::new(
      self.routers,
      self.error_handler,
      self.not_found_handler,
      self.connection_timeout,
    )
  }

  /// This method is equivalent to calling `Arc::new(builder.build())`
  pub fn build_arc(self) -> Arc<HumptyServer> {
    Arc::new(self.build())
  }

  /// Adds a new host sub-app to the server.
  /// The host can contain wildcards, for example `*.example.com`.
  ///
  /// ## Panics
  /// This function will panic if the host is equal to `*`, since this is the default host.
  /// If you want to add a route to every host, simply add it directly to the main app.
  pub fn with_router<T: Router + 'static>(mut self, handler: T) -> Self {
    self.routers.push(Box::new(handler));
    self
  }

  /// Adds a new router to the server and calls the closure with the new router so it can be configured.
  pub fn router<T: FnOnce(HumptyRouterBuilder) -> HumptyRouterBuilder>(self, builder: T) -> Self {
    self.with_router(builder(HumptyRouterBuilder::default()).build())
  }

  /// Sets the error handler for the server.
  pub fn with_error_handler(mut self, handler: ErrorHandler) -> Self {
    self.error_handler = handler;
    self
  }

  /// Sets the not found handler for the server.
  pub fn with_not_found_handler(mut self, handler: NotFoundHandler) -> Self {
    self.not_found_handler = handler;
    self
  }

  /// Sets the connection timeout, the amount of time to wait between keep-alive requests.
  pub fn with_connection_timeout(mut self, timeout: Option<Duration>) -> Self {
    self.connection_timeout = timeout;
    self
  }
}
