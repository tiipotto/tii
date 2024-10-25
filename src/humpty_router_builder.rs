//! Contains the builder for a router

use crate::default_functions::{
  default_error_handler, default_not_found_handler, default_pre_routing_filter,
};
use crate::functional_traits::{
  RequestFilter, RequestHandler, ResponseFilter, RouterFilter, WebsocketHandler,
};
use crate::humpty_builder::{ErrorHandler, NotFoundHandler};
use crate::humpty_router::{HumptyRouter, RouteHandler, WebsocketRouteHandler};
use std::sync::Arc;

/// Represents a sub-app to run for a specific host.
pub struct HumptyRouterBuilder {
  /// This filter/predicate will decide if the router should even serve the request at all
  router_filter: Box<dyn RouterFilter>,

  /// Filters that run before the route is matched.
  /// These filters may modify the path of the request to affect routing decision.
  pre_routing_filters: Vec<Box<dyn RequestFilter>>,
  /// Filters that run once the routing decision has been made.
  /// These filters only run if there is an actual endpoint.
  routing_filters: Vec<Box<dyn RequestFilter>>,

  /// These filters run on the response after the actual endpoint (or the error handler) has been called.
  response_filters: Vec<Box<dyn ResponseFilter>>,

  /// The routes to process requests for and their handlers.
  routes: Vec<RouteHandler>,

  /// The routes to process WebSocket requests for and their handlers.
  websocket_routes: Vec<WebsocketRouteHandler>,

  /// Called when no route has been found in the router.
  not_found_handler: NotFoundHandler,
  /// Called when an error in any of the above occurs.
  error_handler: ErrorHandler,
}

impl Default for HumptyRouterBuilder {
  fn default() -> Self {
    HumptyRouterBuilder {
      router_filter: Box::new(default_pre_routing_filter),
      pre_routing_filters: Vec::default(),
      routing_filters: Vec::default(),
      response_filters: Vec::default(),
      routes: Vec::new(),
      websocket_routes: Vec::new(),
      not_found_handler: default_not_found_handler,
      error_handler: default_error_handler,
    }
  }
}

impl HumptyRouterBuilder {
  /// Create a new sub-app with no routes.
  pub fn new() -> Self {
    HumptyRouterBuilder::default()
  }

  /// Adds a pre routing filter. This is called before any routing is done.
  /// The filter can modify the path in the request to change the outcome of routing.
  /// This filter gets called for every request, even those that later fail to find a handler.
  pub fn with_pre_routing_request_filter<T>(mut self, filter: T) -> Self
  where
    T: RequestFilter + 'static,
  {
    self.pre_routing_filters.push(Box::new(filter));
    self
  }

  /// Adds a routing filter. This filter gets called once routing is done.
  /// This filter is called directly before a handler is called.
  /// This filter is only called on requests that actually do have a handler.
  pub fn with_request_filter<T>(mut self, filter: T) -> Self
  where
    T: RequestFilter + 'static,
  {
    self.routing_filters.push(Box::new(filter));
    self
  }

  /// Adds a response filter. This filter gets called after the response is created.
  /// This response may have been created by:
  /// 1. a pre routing filter
  /// 2. a routing filter
  /// 3. a handler/endpoint
  /// 4. the error handler
  /// 5. the not found handler
  ///
  /// # Note on Errors:
  /// If the response filter returns an error itself then this will cause invocation of the error handler,
  /// even if the error handler was already called previously for the same request.
  /// However, each "request" will only trigger exactly 1 invocation of the response filter so it is not possible
  /// to create a loop between response filter and error handler.
  pub fn with_response_filter<T>(mut self, filter: T) -> Self
  where
    T: ResponseFilter + 'static,
  {
    self.response_filters.push(Box::new(filter));
    self
  }

  /// Adds a route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/blog/*`.
  pub fn with_route<T>(mut self, route: &str, handler: T) -> Self
  where
    T: RequestHandler + 'static,
  {
    self.routes.push(RouteHandler { route: route.to_string(), handler: Box::new(handler) });
    self
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed the stream and the request which triggered its calling.
  pub fn with_websocket_route<T>(mut self, route: &str, handler: T) -> Self
  where
    T: WebsocketHandler + 'static,
  {
    self
      .websocket_routes
      .push(WebsocketRouteHandler { route: route.to_string(), handler: Box::new(handler) });
    self
  }

  /// Build the router
  pub fn build(self) -> HumptyRouter {
    HumptyRouter::new(
      self.router_filter,
      self.pre_routing_filters,
      self.routing_filters,
      self.response_filters,
      self.routes,
      self.websocket_routes,
      self.not_found_handler,
      self.error_handler,
    )
  }

  /// Equivalent of calling Arc::new(builder.build())
  pub fn build_arc(self) -> Arc<HumptyRouter> {
    Arc::new(self.build())
  }
}
