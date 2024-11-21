//! Contains the impl of the router.

use crate::functional_traits::{
  RequestFilter, RequestHandler, ResponseFilter, Router, RouterFilter, WebsocketHandler,
};
use crate::http::method::Method;
use crate::http::mime::{AcceptMimeType, QValue};
use crate::http::request_context::RequestContext;
use crate::http::Response;
use crate::humpty_builder::{ErrorHandler, NotRouteableHandler};
use crate::humpty_error::{HumptyError, HumptyResult};
use crate::stream::ConnectionStream;
use crate::{krauss, trace_log, util};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Encapsulates a route and its handler.
pub struct RouteHandler {
  /// The route that this handler will match.
  pub route: String,
  /// The handler to run when the route is matched.
  pub handler: Box<dyn RequestHandler>,

  /// The method this route will handle
  pub method: Method,

  /// The mime types this route can consume
  /// EMPTY SET means this route does not expect a request body.
  pub consumes: HashSet<AcceptMimeType>,

  /// The mime types this route can produce
  /// EMPTY SET means this route will produce a matching body type.
  pub produces: HashSet<AcceptMimeType>,
}

/// Enum that shows information on how a particular request could be routed on a route.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum RoutingDecision {
  /// Routing matches with the given quality.
  Match(QValue),
  /// Path doesnt match.
  PathMismatch,
  /// Path matches, but method doesn't.
  MethodMismatch,
  /// Path and method do match, but the request body cannot be processed by the route.
  MimeMismatch,
  /// Path and method do match, the body can be processed but the response of the endpoint will not be processable by the client.
  AcceptMismatch,
}

impl PartialOrd for RoutingDecision {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for RoutingDecision {
  fn cmp(&self, other: &Self) -> Ordering {
    match (self, other) {
      (RoutingDecision::Match(q1), RoutingDecision::Match(q2)) => q1.cmp(q2),
      (RoutingDecision::Match(_), RoutingDecision::PathMismatch) => Ordering::Greater,
      (RoutingDecision::Match(_), RoutingDecision::MethodMismatch) => Ordering::Greater,
      (RoutingDecision::Match(_), RoutingDecision::MimeMismatch) => Ordering::Greater,
      (RoutingDecision::Match(_), RoutingDecision::AcceptMismatch) => Ordering::Greater,

      (RoutingDecision::PathMismatch, RoutingDecision::Match(_)) => Ordering::Less,
      (RoutingDecision::PathMismatch, RoutingDecision::PathMismatch) => Ordering::Equal,
      (RoutingDecision::PathMismatch, RoutingDecision::MethodMismatch) => Ordering::Less,
      (RoutingDecision::PathMismatch, RoutingDecision::MimeMismatch) => Ordering::Less,
      (RoutingDecision::PathMismatch, RoutingDecision::AcceptMismatch) => Ordering::Less,

      (RoutingDecision::MethodMismatch, RoutingDecision::Match(_)) => Ordering::Less,
      (RoutingDecision::MethodMismatch, RoutingDecision::PathMismatch) => Ordering::Greater,
      (RoutingDecision::MethodMismatch, RoutingDecision::MethodMismatch) => Ordering::Equal,
      (RoutingDecision::MethodMismatch, RoutingDecision::MimeMismatch) => Ordering::Less,
      (RoutingDecision::MethodMismatch, RoutingDecision::AcceptMismatch) => Ordering::Less,

      (RoutingDecision::MimeMismatch, RoutingDecision::Match(_)) => Ordering::Less,
      (RoutingDecision::MimeMismatch, RoutingDecision::PathMismatch) => Ordering::Greater,
      (RoutingDecision::MimeMismatch, RoutingDecision::MethodMismatch) => Ordering::Greater,
      (RoutingDecision::MimeMismatch, RoutingDecision::MimeMismatch) => Ordering::Equal,
      (RoutingDecision::MimeMismatch, RoutingDecision::AcceptMismatch) => Ordering::Less,

      (RoutingDecision::AcceptMismatch, RoutingDecision::Match(_)) => Ordering::Less,
      (RoutingDecision::AcceptMismatch, RoutingDecision::PathMismatch) => Ordering::Greater,
      (RoutingDecision::AcceptMismatch, RoutingDecision::MethodMismatch) => Ordering::Greater,
      (RoutingDecision::AcceptMismatch, RoutingDecision::MimeMismatch) => Ordering::Greater,
      (RoutingDecision::AcceptMismatch, RoutingDecision::AcceptMismatch) => Ordering::Equal,
    }
  }
}

impl RouteHandler {
  /// Checks whether this route matches the given one, respecting its own wildcards only.
  /// For example, `/blog/*` will match `/blog/my-first-post` but not the other way around.
  pub fn matches(&self, route: &RequestContext) -> RoutingDecision {
    let head = route.request_head();

    if !krauss::wildcard_match(self.route.as_str(), head.path()) {
      return RoutingDecision::PathMismatch;
    }

    if &self.method != head.method() {
      return RoutingDecision::MethodMismatch;
    }

    if let Some(content_type) = head.get_content_type() {
      let mut found = false;
      for mime in &self.consumes {
        if mime.permits_specific(content_type) {
          found = true;
          break;
        }
      }

      if !found {
        return RoutingDecision::MimeMismatch;
      }
    }

    if self.produces.is_empty() {
      //The endpoint either doesn't produce a body or declares that it will produce a matching body...
      return RoutingDecision::Match(QValue::MAX);
    }

    let acc = head.get_accept();
    if acc.is_empty() {
      //The client doesn't accept a body.
      return RoutingDecision::MimeMismatch;
    }

    let mut current_q = None;
    for accept in acc {
      for mime in &self.produces {
        if !accept.get_type().permits(mime) {
          continue;
        }

        let qvalue = accept.qvalue();
        if qvalue == QValue::MAX {
          return RoutingDecision::Match(qvalue);
        }

        match &current_q {
          None => {
            current_q = Some(accept.qvalue());
          }
          Some(current) => {
            if current < &qvalue {
              current_q = Some(accept.qvalue());
            }
          }
        }
      }
    }

    if let Some(qval) = current_q {
      return RoutingDecision::Match(qval);
    }

    RoutingDecision::AcceptMismatch
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
  not_found_handler: NotRouteableHandler,

  /// Called when no acceptable route has been found
  not_acceptable_handler: NotRouteableHandler,
  /// Called when no route with a handled method has been found.
  method_not_allowed_handler: NotRouteableHandler,
  /// Called when no route with a given media type has been found.
  unsupported_media_type_handler: NotRouteableHandler,

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
    not_found_handler: NotRouteableHandler,
    not_acceptable_handler: NotRouteableHandler,
    method_not_allowed_handler: NotRouteableHandler,
    unsupported_media_type_handler: NotRouteableHandler,
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
      not_acceptable_handler,
      method_not_allowed_handler,
      unsupported_media_type_handler,
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

    let mut best_decision = RoutingDecision::PathMismatch;
    let mut best_handler = None;

    for handler in &self.routes {
      let decision = handler.matches(request);
      if best_decision >= decision {
        continue;
      }

      best_decision = decision;
      if let RoutingDecision::Match(qv) = decision {
        best_handler = Some(handler);
        if qv == QValue::MAX {
          break;
        }
      }
    }

    if let Some(handler) = best_handler {
      request.set_routed_path(handler.route.as_str());
      for filter in self.routing_filters.iter() {
        if let Some(resp) = filter.filter(request)? {
          return Ok(resp);
        }
      }

      return handler.handler.serve(request);
    }

    match best_decision {
      RoutingDecision::PathMismatch => (self.not_found_handler)(request, &self.routes),
      RoutingDecision::MethodMismatch => (self.method_not_allowed_handler)(request, &self.routes),
      RoutingDecision::MimeMismatch => (self.unsupported_media_type_handler)(request, &self.routes),
      RoutingDecision::AcceptMismatch => (self.not_acceptable_handler)(request, &self.routes),
      // We found a handler! Why are we here?
      RoutingDecision::Match(_) => util::unreachable(),
    }
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
