//! Contains the impl of the router.

use crate::functional_traits::{
  RequestFilter, RequestHandler, ResponseFilter, Router, RouterFilter, WebsocketHandler,
};
use crate::http::request_context::RequestContext;
use crate::http::Response;
use crate::humpty_builder::{ErrorHandler, NotFoundHandler};
use crate::humpty_error::{HumptyError, HumptyResult};
use crate::stream::ConnectionStream;
use crate::{krauss, trace_log};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Encapsulates a route and its handler.
pub struct RouteHandler {
  /// The route that this handler will match.
  pub route: String,
  /// The handler to run when the route is matched.
  pub handler: Box<dyn RequestHandler>,
}

impl RouteHandler {
  /// Checks whether this route matches the given one, respecting its own wildcards only.
  /// For example, `/blog/*` will match `/blog/my-first-post` but not the other way around.
  pub fn route_matches(&self, route: &str) -> bool {
    krauss::wildcard_match(self.route.as_str(), route)
  }
}

impl Debug for RouteHandler {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("RouteHandler({})", self.route.as_str()))
  }
}

/// Encapsulates a route and its WebSocket handler.
pub struct WebsocketRouteHandler {
  /// The route that this handler will match.
  pub route: String,
  /// The handler to run when the route is matched.
  pub handler: Box<dyn WebsocketHandler>,
}

impl Debug for WebsocketRouteHandler {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("WebsocketRouteHandler({})", self.route.as_str()))
  }
}

impl WebsocketRouteHandler {
  /// Checks whether this route matches the given one, respecting its own wildcards only.
  /// For example, `/blog/*` will match `/blog/my-first-post` but not the other way around.
  pub fn route_matches(&self, route: &str) -> bool {
    krauss::wildcard_match(self.route.as_str(), route)
  }
}

/// Represents a sub-app to run for a specific host.
pub struct HumptyRouter {
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

impl Debug for HumptyRouter {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("HumptyRouter(pre_routing_filters={}, routing_filters={}, response_filters={}, routes={:?}, websocket_routes={})",
                                 self.pre_routing_filters.len(),
            self.routing_filters.len(),
            self.response_filters.len(),
            self.routes,
            self.websocket_routes.len(),
        ))
  }
}

impl HumptyRouter {
  #[allow(clippy::too_many_arguments)] //Only called by the builder.
  pub(crate) fn new(
    router_filter: Box<dyn RouterFilter>,
    pre_routing_filters: Vec<Box<dyn RequestFilter>>,
    routing_filters: Vec<Box<dyn RequestFilter>>,
    response_filters: Vec<Box<dyn ResponseFilter>>,
    routes: Vec<RouteHandler>,
    websocket_routes: Vec<WebsocketRouteHandler>,
    not_found_handler: NotFoundHandler,
    error_handler: ErrorHandler,
  ) -> Self {
    Self {
      router_filter,
      pre_routing_filters,
      routing_filters,
      response_filters,
      routes,
      websocket_routes,
      not_found_handler,
      error_handler,
    }
  }

  fn serve_ws(
    &self,
    stream: &dyn ConnectionStream,
    request: &mut RequestContext,
  ) -> HumptyResult<bool> {
    if !self.router_filter.filter(request)? {
      return Ok(false);
    }

    if let Some(handler) = self
      .websocket_routes // Get the WebSocket routes of the sub-app
      .iter() // Iterate over the routes
      .find(|route| route.route_matches(request.request_head().path()))
    {
      handler.handler.serve(request.request_head().clone(), stream.new_ref());
      return Ok(true);
    }

    // TODO how can I tell a websocket request gracefully that there is no one here for it? HTTP 404?, this just shuts the socket.
    trace_log!("WebsocketConnectionClosed Not found");
    Ok(true)
  }

  fn call_error_handler(
    &self,
    request: &mut RequestContext,
    error: HumptyError,
  ) -> HumptyResult<Response> {
    //TODO i am not 100% sure this is a good idea, but it probably is a good idea.
    //The only thing i could consider is having the default impl do this and outsource this responsibility to the user
    //Not doing this on io::Errors when reading the request body will cause stuff to break in a horrific manner.
    //So always doing this is safer, but prevents keepalive in cases where the error is unrelated to the http stream
    //and the user properly handles it.
    request.force_connection_close();

    (self.error_handler)(request, error)
  }

  fn serve_outer(&self, request: &mut RequestContext) -> HumptyResult<Option<Response>> {
    if !self.router_filter.filter(request)? {
      return Ok(None);
    }

    let mut resp = self.serve_inner(request).or_else(|e| self.call_error_handler(request, e))?;
    for filter in self.response_filters.iter() {
      resp = filter.filter(request, resp).or_else(|e| self.call_error_handler(request, e))?;
    }

    Ok(Some(resp))
  }
  fn serve_inner(&self, request: &mut RequestContext) -> HumptyResult<Response> {
    for filter in self.pre_routing_filters.iter() {
      if let Some(resp) = filter.filter(request)? {
        return Ok(resp);
      }
    }

    if let Some(handler) = self
      .routes // Get the routes of the sub-app
      .iter() // Iterate over the routes
      .find(|route| route.route_matches(request.request_head().path()))
    {
      request.set_routed_path(handler.route.as_str());

      for filter in self.routing_filters.iter() {
        if let Some(resp) = filter.filter(request)? {
          return Ok(resp);
        }
      }

      return handler.handler.serve(request);
    }

    (self.not_found_handler)(request)
  }
}

impl Router for HumptyRouter {
  fn serve(&self, request: &mut RequestContext) -> HumptyResult<Option<Response>> {
    self.serve_outer(request)
  }

  fn serve_websocket(
    &self,
    stream: &dyn ConnectionStream,
    request: &mut RequestContext,
  ) -> HumptyResult<bool> {
    self.serve_ws(stream, request)
  }
}

impl Router for Arc<HumptyRouter> {
  fn serve(&self, request: &mut RequestContext) -> HumptyResult<Option<Response>> {
    Arc::as_ref(self).serve(request)
  }

  fn serve_websocket(
    &self,
    stream: &dyn ConnectionStream,
    request: &mut RequestContext,
  ) -> HumptyResult<bool> {
    Arc::as_ref(self).serve_websocket(stream, request)
  }
}
