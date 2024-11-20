//! Contains the builder for a router

use crate::default_functions::{
  default_error_handler, default_method_not_allowed_handler, default_not_acceptable_handler,
  default_not_found_handler, default_pre_routing_filter, default_unsupported_media_type_handler,
};
use crate::functional_traits::{
  RequestFilter, RequestHandler, ResponseFilter, RouterFilter, WebsocketHandler,
};
use crate::http::method::Method;
use crate::http::mime::AcceptMimeType;
use crate::http::request_context::RequestContext;
use crate::http::Response;
use crate::humpty_builder::{ErrorHandler, NotRouteableHandler};
use crate::humpty_error::HumptyResult;
use crate::humpty_router::{HumptyRouter, RouteHandler, WebsocketRouteHandler};
use std::collections::HashSet;
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
  not_found_handler: NotRouteableHandler,

  not_acceptable_handler: NotRouteableHandler,
  method_not_allowed_handler: NotRouteableHandler,
  unsupported_media_type_handler: NotRouteableHandler,

  /// Called when an error in any of the above occurs.
  error_handler: ErrorHandler,
}

/// For multi method routes!
#[derive(Debug)]
struct RouteWrapper<T: RequestHandler + 'static>(Arc<T>);
impl<T: RequestHandler + 'static> RequestHandler for RouteWrapper<T> {
  fn serve(&self, request: &RequestContext) -> HumptyResult<Response> {
    self.0.serve(request)
  }
}

impl<T: RequestHandler + 'static> Clone for RouteWrapper<T> {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

/// Builder for a route/endpoint.
pub struct HumptyRouteBuilder {
  inner: HumptyRouterBuilder,
  route: String,
  method: Method,
  consumes: HashSet<AcceptMimeType>,
  produces: HashSet<AcceptMimeType>,
}

impl HumptyRouteBuilder {
  pub(crate) fn new(
    router_builder: HumptyRouterBuilder,
    method: Method,
    route: String,
  ) -> HumptyRouteBuilder {
    HumptyRouteBuilder {
      inner: router_builder,
      route,
      method,
      consumes: Default::default(),
      produces: Default::default(),
    }
  }

  /// Add a mime type which the endpoint can consume.
  pub fn consumes(mut self, mime: impl Into<AcceptMimeType>) -> Self {
    self.consumes.insert(mime.into());
    self
  }

  /// Add a mime type which the endpoint may produce.
  pub fn produces(mut self, mime: impl Into<AcceptMimeType>) -> Self {
    self.consumes.insert(mime.into());
    self
  }

  /// Finish building the route by proving the route.
  pub fn endpoint<T: RequestHandler + 'static>(mut self, handler: T) -> HumptyRouterBuilder {
    self.inner.routes.push(RouteHandler {
      route: self.route,
      handler: Box::new(handler),
      method: self.method,
      consumes: self.consumes,
      produces: self.produces,
    });
    self.inner
  }
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
      not_acceptable_handler: default_not_acceptable_handler,
      method_not_allowed_handler: default_method_not_allowed_handler,
      unsupported_media_type_handler: default_unsupported_media_type_handler,
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

  /// Adds a route that will handle all well known reasonable http methods.
  /// - GET
  /// - PUT
  /// - POST
  /// - PATCH
  /// - DELETE
  /// - OPTIONS
  ///
  /// The endpoint will be called for any media type.
  pub fn route_any<T>(self, route: &str, handler: T) -> Self
  where
    T: RequestHandler + 'static,
  {
    let wrapped = RouteWrapper(Arc::new(handler));

    self
      .route_get(route, wrapped.clone())
      .route_put(route, wrapped.clone())
      .route_post(route, wrapped.clone())
      .route_patch(route, wrapped.clone())
      .route_delete(route, wrapped.clone())
      .route_options(route, wrapped)
  }

  /// Adds a route that will handle the given http method.
  /// The endpoint will be called for any media type.
  pub fn route_method<T: RequestHandler + 'static>(
    mut self,
    method: Method,
    route: &str,
    handler: T,
  ) -> Self {
    self.routes.push(RouteHandler {
      route: route.to_string(),
      handler: Box::new(handler),
      method,
      consumes: HashSet::from([AcceptMimeType::Wildcard]),
      produces: HashSet::new(),
    });
    self
  }

  /// Adds a route that will handle the GET http method.
  /// The endpoint will be called for any media type.
  pub fn route_get<T: RequestHandler + 'static>(self, route: &str, handler: T) -> Self {
    self.route_method(Method::Get, route, handler)
  }

  /// Adds a route that will handle the POST http method.
  /// The endpoint will be called for any media type.
  pub fn route_post<T: RequestHandler + 'static>(self, route: &str, handler: T) -> Self {
    self.route_method(Method::Post, route, handler)
  }

  /// Adds a route that will handle the PUT http method.
  /// The endpoint will be called for any media type.
  pub fn route_put<T: RequestHandler + 'static>(self, route: &str, handler: T) -> Self {
    self.route_method(Method::Put, route, handler)
  }

  /// Adds a route that will handle the PATCH http method.
  /// The endpoint will be called for any media type.
  pub fn route_patch<T: RequestHandler + 'static>(self, route: &str, handler: T) -> Self {
    self.route_method(Method::Patch, route, handler)
  }

  /// Adds a route that will handle the DELETE http method.
  /// The endpoint will be called for any media type.
  pub fn route_delete<T: RequestHandler + 'static>(self, route: &str, handler: T) -> Self {
    self.route_method(Method::Delete, route, handler)
  }

  /// Adds a route that will handle the OPTIONS http method.
  /// The endpoint will be called for any media type.
  pub fn route_options<T: RequestHandler + 'static>(self, route: &str, handler: T) -> Self {
    self.route_method(Method::Options, route, handler)
  }

  /// Helper fn that will just call the passed closure,
  /// this can be used to write the builder in an indenting way.
  /// This method is purely cosmetic.
  pub fn begin<T: FnOnce(Self) -> Self>(self, section: T) -> Self {
    section(self)
  }

  /// Build an endpoint with a GET http method.
  pub fn get(self, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, Method::Get, route.to_string())
  }

  /// Build an endpoint with a GET http method.
  pub fn begin_get<T: FnOnce(HumptyRouteBuilder) -> Self>(self, route: &str, closure: T) -> Self {
    closure(self.get(route))
  }

  /// Build an endpoint with a POST http method.
  pub fn post(self, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, Method::Post, route.to_string())
  }

  /// Build an endpoint with a POST http method.
  pub fn begin_post<T: FnOnce(HumptyRouteBuilder) -> Self>(self, route: &str, closure: T) -> Self {
    closure(self.post(route))
  }

  /// Build an endpoint with a PUT http method.
  pub fn put(self, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, Method::Put, route.to_string())
  }

  /// Build an endpoint with a PUT http method.
  pub fn begin_put<T: FnOnce(HumptyRouteBuilder) -> Self>(self, route: &str, closure: T) -> Self {
    closure(self.put(route))
  }

  /// Build an endpoint with a PATCH http method.
  pub fn patch(self, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, Method::Patch, route.to_string())
  }

  /// Build an endpoint with a PATCH http method.
  pub fn begin_patch<T: FnOnce(HumptyRouteBuilder) -> Self>(self, route: &str, closure: T) -> Self {
    closure(self.patch(route))
  }

  /// Build an endpoint with a DELETE http method.
  pub fn delete(self, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, Method::Delete, route.to_string())
  }

  /// Build an endpoint with a DELETE http method.
  pub fn begin_delete<T: FnOnce(HumptyRouteBuilder) -> Self>(
    self,
    route: &str,
    closure: T,
  ) -> Self {
    closure(self.delete(route))
  }

  /// Build an endpoint with a OPTIONS http method.
  pub fn options(self, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, Method::Options, route.to_string())
  }

  /// Build an endpoint with a OPTIONS http method.
  pub fn begin_options<T: FnOnce(HumptyRouteBuilder) -> Self>(
    self,
    route: &str,
    closure: T,
  ) -> Self {
    closure(self.delete(route))
  }

  /// Build an endpoint with a less commonly used or custom http method.
  pub fn method(self, method: Method, route: &str) -> HumptyRouteBuilder {
    HumptyRouteBuilder::new(self, method, route.to_string())
  }

  /// Build an endpoint with a less commonly used or custom http method.
  pub fn begin_method<T: FnOnce(HumptyRouteBuilder) -> Self>(
    self,
    method: Method,
    route: &str,
    closure: T,
  ) -> Self {
    closure(self.method(method, route))
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
      self.not_acceptable_handler,
      self.method_not_allowed_handler,
      self.unsupported_media_type_handler,
      self.error_handler,
    )
  }

  /// Equivalent of calling Arc::new(builder.build())
  pub fn build_arc(self) -> Arc<HumptyRouter> {
    Arc::new(self.build())
  }
}
