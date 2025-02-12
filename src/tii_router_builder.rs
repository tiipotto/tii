//! Contains the builder for a router

use crate::default_functions::{
  default_error_handler, default_method_not_allowed_handler, default_not_acceptable_handler,
  default_not_found_handler, default_pre_routing_filter, default_unsupported_media_type_handler,
};
use crate::functional_traits::{
  HttpEndpoint, RequestFilter, ResponseFilter, RouterFilter, WebsocketEndpoint,
};
use crate::AcceptMimeType;
use crate::HttpMethod;
use crate::RequestContext;
use crate::TiiResult;
use crate::{DefaultRouter, Response, Router};
use crate::{ErrorHandler, NotRouteableHandler};
use crate::{HttpRoute, WebSocketRoute};
use crate::{WebsocketReceiver, WebsocketSender};
use std::collections::HashSet;
use std::sync::Arc;

/// Represents a sub-app to run for a specific host.
pub struct RouterBuilder {
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
  routes: Vec<HttpRoute>,

  /// The routes to process WebSocket requests for and their handlers.
  websocket_routes: Vec<WebSocketRoute>,

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
struct RouteWrapper<T: HttpEndpoint + 'static>(Arc<T>);
impl<T: HttpEndpoint + 'static> HttpEndpoint for RouteWrapper<T> {
  fn serve(&self, request: &RequestContext) -> TiiResult<Response> {
    self.0.serve(request)
  }
}

impl<T: HttpEndpoint + 'static> Clone for RouteWrapper<T> {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

/// For multi method routes!
#[derive(Debug)]
struct WsRouteWrapper<T: WebsocketEndpoint + 'static>(Arc<T>);
impl<T: WebsocketEndpoint + 'static> WebsocketEndpoint for WsRouteWrapper<T> {
  fn serve(
    &self,
    request: &RequestContext,
    receiver: WebsocketReceiver,
    sender: WebsocketSender,
  ) -> TiiResult<()> {
    self.0.serve(request, receiver, sender)
  }
}

impl<T: WebsocketEndpoint + 'static> Clone for WsRouteWrapper<T> {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

/// Builder for a route/endpoint.
pub struct RouteBuilder {
  inner: RouterBuilder,
  route: String,
  method: HttpMethod,
  consumes: HashSet<AcceptMimeType>,
  produces: HashSet<AcceptMimeType>,
}

impl RouteBuilder {
  pub(crate) fn new(
    router_builder: RouterBuilder,
    method: HttpMethod,
    route: String,
  ) -> RouteBuilder {
    RouteBuilder {
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
    self.produces.insert(mime.into());
    self
  }

  /// Finish building the route by proving the route.
  pub fn endpoint<T: HttpEndpoint + 'static>(
    mut self,
    handler: T,
  ) -> TiiResult<RouterBuilder> {
    self.inner.routes.push(HttpRoute::new(
      self.route,
      self.method,
      self.consumes,
      self.produces,
      handler,
    )?);
    Ok(self.inner)
  }
}

impl Default for RouterBuilder {
  fn default() -> Self {
    RouterBuilder {
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

impl RouterBuilder {
  /// Create a new sub-app with no routes.
  pub fn new() -> Self {
    RouterBuilder::default()
  }

  /// Adds a pre routing filter. This is called before any routing is done.
  /// The filter can modify the path in the request to change the outcome of routing.
  /// This filter gets called for every request, even those that later fail to find a handler.
  pub fn with_pre_routing_request_filter<T>(mut self, filter: T) -> TiiResult<Self>
  where
    T: RequestFilter + 'static,
  {
    self.pre_routing_filters.push(Box::new(filter));
    Ok(self)
  }

  /// Adds a routing filter. This filter gets called once routing is done.
  /// This filter is called directly before a handler is called.
  /// This filter is only called on requests that actually do have a handler.
  pub fn with_request_filter<T>(mut self, filter: T) -> TiiResult<Self>
  where
    T: RequestFilter + 'static,
  {
    self.routing_filters.push(Box::new(filter));
    Ok(self)
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
  pub fn with_response_filter<T>(mut self, filter: T) -> TiiResult<Self>
  where
    T: ResponseFilter + 'static,
  {
    self.response_filters.push(Box::new(filter));
    Ok(self)
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
  pub fn route_any<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: HttpEndpoint + 'static,
  {
    let wrapped = RouteWrapper(Arc::new(handler));

    self
      .route_get(route, wrapped.clone())?
      .route_put(route, wrapped.clone())?
      .route_post(route, wrapped.clone())?
      .route_patch(route, wrapped.clone())?
      .route_delete(route, wrapped.clone())?
      .route_options(route, wrapped)
  }

  /// Helper fn to make some builder code look a bit cleaner.
  pub const fn ok(self) -> TiiResult<Self> {
    Ok(self)
  }

  /// Adds a route that will handle the given http method.
  /// The endpoint will be called for any media type.
  pub fn route_method<T: HttpEndpoint + 'static>(
    mut self,
    method: HttpMethod,
    route: &str,
    handler: T,
  ) -> TiiResult<Self> {
    self.routes.push(HttpRoute::new(
      route,
      method,
      HashSet::from([AcceptMimeType::Wildcard]),
      HashSet::new(),
      handler,
    )?);
    Ok(self)
  }

  /// Adds a route that will handle the GET http method.
  /// The endpoint will be called for any media type.
  pub fn route_get<T: HttpEndpoint + 'static>(self, route: &str, handler: T) -> TiiResult<Self> {
    self.route_method(HttpMethod::Get, route, handler)
  }

  /// Adds a route that will handle the POST http method.
  /// The endpoint will be called for any media type.
  pub fn route_post<T: HttpEndpoint + 'static>(
    self,
    route: &str,
    handler: T,
  ) -> TiiResult<Self> {
    self.route_method(HttpMethod::Post, route, handler)
  }

  /// Adds a route that will handle the PUT http method.
  /// The endpoint will be called for any media type.
  pub fn route_put<T: HttpEndpoint + 'static>(self, route: &str, handler: T) -> TiiResult<Self> {
    self.route_method(HttpMethod::Put, route, handler)
  }

  /// Adds a route that will handle the PATCH http method.
  /// The endpoint will be called for any media type.
  pub fn route_patch<T: HttpEndpoint + 'static>(
    self,
    route: &str,
    handler: T,
  ) -> TiiResult<Self> {
    self.route_method(HttpMethod::Patch, route, handler)
  }

  /// Adds a route that will handle the DELETE http method.
  /// The endpoint will be called for any media type.
  pub fn route_delete<T: HttpEndpoint + 'static>(
    self,
    route: &str,
    handler: T,
  ) -> TiiResult<Self> {
    self.route_method(HttpMethod::Delete, route, handler)
  }

  /// Adds a route that will handle the OPTIONS http method.
  /// The endpoint will be called for any media type.
  pub fn route_options<T: HttpEndpoint + 'static>(
    self,
    route: &str,
    handler: T,
  ) -> TiiResult<Self> {
    self.route_method(HttpMethod::Options, route, handler)
  }

  /// Helper fn that will just call the passed closure,
  /// this can be used to write the builder in an indenting way.
  /// This method is purely cosmetic.
  pub fn begin<T: FnOnce(Self) -> TiiResult<Self>>(self, section: T) -> TiiResult<Self> {
    section(self)
  }

  /// Build an endpoint with a GET http method.
  pub fn get(self, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, HttpMethod::Get, route.to_string())
  }

  /// Build an endpoint with a GET http method.
  pub fn begin_get<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.get(route))
  }

  /// Build an endpoint with a POST http method.
  pub fn post(self, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, HttpMethod::Post, route.to_string())
  }

  /// Build an endpoint with a POST http method.
  pub fn begin_post<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.post(route))
  }

  /// Build an endpoint with a PUT http method.
  pub fn put(self, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, HttpMethod::Put, route.to_string())
  }

  /// Build an endpoint with a PUT http method.
  pub fn begin_put<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.put(route))
  }

  /// Build an endpoint with a PATCH http method.
  pub fn patch(self, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, HttpMethod::Patch, route.to_string())
  }

  /// Build an endpoint with a PATCH http method.
  pub fn begin_patch<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.patch(route))
  }

  /// Build an endpoint with a DELETE http method.
  pub fn delete(self, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, HttpMethod::Delete, route.to_string())
  }

  /// Build an endpoint with a DELETE http method.
  pub fn begin_delete<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.delete(route))
  }

  /// Build an endpoint with a OPTIONS http method.
  pub fn options(self, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, HttpMethod::Options, route.to_string())
  }

  /// Build an endpoint with a OPTIONS http method.
  pub fn begin_options<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.delete(route))
  }

  /// Build an endpoint with a less commonly used or custom http method.
  pub fn method(self, method: HttpMethod, route: &str) -> RouteBuilder {
    RouteBuilder::new(self, method, route.to_string())
  }

  /// Build an endpoint with a less commonly used or custom http method.
  pub fn begin_method<T: FnOnce(RouteBuilder) -> TiiResult<Self>>(
    self,
    method: HttpMethod,
    route: &str,
    closure: T,
  ) -> TiiResult<Self> {
    closure(self.method(method, route))
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will be called for any commonly used HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method.
  pub fn ws_route_any<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    let wrapped = WsRouteWrapper(Arc::new(handler));

    self
      .ws_route_get(route, wrapped.clone())?
      .ws_route_put(route, wrapped.clone())?
      .ws_route_post(route, wrapped.clone())?
      .ws_route_patch(route, wrapped.clone())?
      .ws_route_delete(route, wrapped.clone())?
      .ws_route_options(route, wrapped)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the specified HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method.
  pub fn ws_route_method<T: WebsocketEndpoint + 'static>(
    mut self,
    method: HttpMethod,
    route: &str,
    handler: T,
  ) -> TiiResult<Self> {
    self.websocket_routes.push(WebSocketRoute::new(
      route,
      method,
      HashSet::new(),
      HashSet::new(),
      handler,
    )?);
    Ok(self)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the GET HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method and will call this endpoint.
  pub fn ws_route_get<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    self.ws_route_method(HttpMethod::Get, route, handler)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the POST HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method and will NOT call this endpoint.
  pub fn ws_route_post<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    self.ws_route_method(HttpMethod::Post, route, handler)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the PUT HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method and will NOT call this endpoint.
  pub fn ws_route_put<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    self.ws_route_method(HttpMethod::Put, route, handler)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the OPTIONS HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method and will NOT call this endpoint.
  pub fn ws_route_options<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    self.ws_route_method(HttpMethod::Options, route, handler)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the PATCH HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method and will NOT call this endpoint.
  pub fn ws_route_patch<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    self.ws_route_method(HttpMethod::Patch, route, handler)
  }

  /// Adds a WebSocket route and associated handler to the sub-app.
  /// Routes can include wildcards, for example `/ws/*`.
  /// The handler is passed a reading and writing end of the websocket.
  /// The endpoint will only listen for HTTP upgrade requests that use the DELETE HTTP method.
  /// Ordinary Web-Socket clients only use the GET Method and will NOT call this endpoint.
  pub fn ws_route_delete<T>(self, route: &str, handler: T) -> TiiResult<Self>
  where
    T: WebsocketEndpoint + 'static,
  {
    self.ws_route_method(HttpMethod::Delete, route, handler)
  }

  /// Build the router
  pub fn build(self) -> impl Router + 'static {
    DefaultRouter::new(
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
  pub fn build_arc(self) -> Arc<impl Router + 'static> {
    Arc::new(self.build())
  }
}
