//! Contains the impl of the router.

use crate::functional_traits::{
  TiiHttpEndpoint, TiiRequestFilter, TiiResponseFilter, TiiRouter, TiiRouterFilter,
  TiiRouterWebSocketServingResponse, TiiWebsocketEndpoint,
};
use crate::stream::TiiConnectionStream;
use crate::tii_builder::{ErrorHandler, NotRouteableHandler};
use crate::tii_error::{InvalidPathError, RequestHeadParsingError, TiiError, TiiResult};
use crate::util::unwrap_some;
use crate::TiiHttpHeaderName;
use crate::TiiHttpMethod;
use crate::TiiHttpVersion;
use crate::TiiRequestContext;
use crate::{trace_log, util};
use crate::{TiiAcceptMimeType, TiiQValue};
use crate::{TiiResponse, TiiStatusCode};
use base64::Engine;
use regex::{Error, Regex};
use sha1::{Digest, Sha1};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

#[derive(Debug, Clone)]
enum PathPart {
  Literal(String),
  Variable(String),
  Wildcard,
  RegexVariable(String, Regex),
  RegexTailVariable(String, Regex),
}

impl PathPart {
  fn parse(path: impl AsRef<str>) -> TiiResult<Vec<PathPart>> {
    let mut path = path.as_ref();
    let full_path = path;
    if path == "/" || path.is_empty() {
      return Ok(Vec::new());
    }

    if path.starts_with("/") {
      path = &path[1..];
    }

    let mut parts = Vec::new();
    loop {
      if path.is_empty() || path == "/" {
        return Ok(parts);
      }

      let part = if let Some(idx) = path.find("/") {
        let part = &path[0..idx];
        path = &path[idx + 1..];
        part
      } else {
        let part = path;
        path = "";
        part
      };

      if part == "*" {
        parts.push(PathPart::Wildcard);
        if !path.is_empty() && path != "/" {
          return Err(InvalidPathError::MorePartsAfterWildcard(full_path.to_string()).into());
        }
        return Ok(parts);
      }

      if part.starts_with("{") && part.ends_with("}") {
        let variable = &part[1..part.len() - 1];
        if let Some((name, regex)) = variable.split_once(":") {
          let reg = Regex::new(regex).map_err(|e| match e {
            Error::Syntax(syntax) => {
              InvalidPathError::RegexSyntaxError(full_path.to_string(), regex.to_string(), syntax)
            }
            Error::CompiledTooBig(limit) => {
              InvalidPathError::RegexTooBig(full_path.to_string(), regex.to_string(), limit)
            }
            _ => InvalidPathError::RegexSyntaxError(
              full_path.to_string(),
              regex.to_string(),
              e.to_string(),
            ),
          })?;
          if !path.is_empty() && path != "/" {
            parts.push(PathPart::RegexVariable(name.to_string(), reg));
            continue;
          }

          parts.push(PathPart::RegexTailVariable(name.to_string(), reg));
          continue;
        }

        parts.push(PathPart::Variable(variable.to_string()));
        continue;
      }

      parts.push(PathPart::Literal(part.to_string()));
    }
  }

  const fn is_tail(&self) -> bool {
    matches!(self, PathPart::Wildcard | PathPart::RegexTailVariable(_, _))
  }
  fn matches(
    &self,
    part: &str,
    remaining: &str,
    variables: &mut Option<HashMap<String, String>>,
  ) -> bool {
    match self {
      PathPart::Literal(literal) => part == literal.as_str(),
      PathPart::Variable(var_name) => {
        if variables.is_none() {
          variables.replace(HashMap::new());
        }

        unwrap_some(variables.as_mut()).insert(var_name.to_string(), part.to_string());
        true
      }
      PathPart::Wildcard => true,
      PathPart::RegexVariable(var_name, regex) => {
        if regex.is_match(part) {
          if variables.is_none() {
            variables.replace(HashMap::new());
          }

          unwrap_some(variables.as_mut()).insert(var_name.to_string(), part.to_string());
          return true;
        }
        false
      }
      PathPart::RegexTailVariable(var_name, regex) => {
        if regex.is_match(remaining) {
          if variables.is_none() {
            variables.replace(HashMap::new());
          }

          unwrap_some(variables.as_mut()).insert(var_name.to_string(), remaining.to_string());
          return true;
        }
        false
      }
    }
  }
}

#[derive(Debug, Clone)]
/// Encapsulates a route and its handler.
pub struct TiiRouteable {
  /// The route that this handler will match.
  path: String,

  parts: Vec<PathPart>,

  /// The method this route will handle
  method: TiiHttpMethod,

  /// The mime types this route can consume
  /// EMPTY SET means this route does not expect a request body.
  consumes: HashSet<TiiAcceptMimeType>,

  /// The mime types this route can produce
  /// EMPTY SET means this route will produce a matching body type.
  produces: HashSet<TiiAcceptMimeType>,
}

pub(crate) struct HttpRoute {
  pub(crate) routeable: TiiRouteable,

  /// The handler to run when the route is matched.
  pub(crate) handler: Box<dyn TiiHttpEndpoint>,
}

pub(crate) struct WebSocketRoute {
  pub(crate) routeable: TiiRouteable,

  /// The handler to run when the route is matched.
  pub(crate) handler: Box<dyn TiiWebsocketEndpoint>,
}

impl HttpRoute {
  pub(crate) fn new(
    path: impl ToString,
    method: impl Into<TiiHttpMethod>,
    consumes: HashSet<TiiAcceptMimeType>,
    produces: HashSet<TiiAcceptMimeType>,
    route: impl TiiHttpEndpoint + 'static,
  ) -> TiiResult<Self> {
    Ok(HttpRoute {
      routeable: TiiRouteable::new(path, method, consumes, produces)?,
      handler: Box::new(route) as Box<dyn TiiHttpEndpoint>,
    })
  }
}

impl WebSocketRoute {
  pub(crate) fn new(
    path: impl ToString,
    method: impl Into<TiiHttpMethod>,
    consumes: HashSet<TiiAcceptMimeType>,
    produces: HashSet<TiiAcceptMimeType>,
    route: impl TiiWebsocketEndpoint + 'static,
  ) -> TiiResult<Self> {
    Ok(WebSocketRoute {
      routeable: TiiRouteable::new(path, method, consumes, produces)?,
      handler: Box::new(route) as Box<dyn TiiWebsocketEndpoint>,
    })
  }
}

/// Enum that shows information on how a particular request could be routed on a route.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum TiiRoutingDecision {
  /// Routing matches with the given quality and path params.
  Match(TiiQValue, Option<HashMap<String, String>>),
  /// Path doesnt match.
  PathMismatch,
  /// Path matches, but method doesn't.
  MethodMismatch,
  /// Path and method do match, but the request body cannot be processed by the route.
  MimeMismatch,
  /// Path and method do match, the body can be processed but the response of the endpoint will not be processable by the client.
  AcceptMismatch,
}

impl Display for TiiRoutingDecision {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //TODO make this not shit
    Debug::fmt(self, f)
  }
}

impl PartialOrd for TiiRoutingDecision {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TiiRoutingDecision {
  fn cmp(&self, other: &Self) -> Ordering {
    match (self, other) {
      (TiiRoutingDecision::Match(q1, _), TiiRoutingDecision::Match(q2, _)) => q1.cmp(q2),
      (TiiRoutingDecision::Match(_, _), TiiRoutingDecision::PathMismatch) => Ordering::Greater,
      (TiiRoutingDecision::Match(_, _), TiiRoutingDecision::MethodMismatch) => Ordering::Greater,
      (TiiRoutingDecision::Match(_, _), TiiRoutingDecision::MimeMismatch) => Ordering::Greater,
      (TiiRoutingDecision::Match(_, _), TiiRoutingDecision::AcceptMismatch) => Ordering::Greater,

      (TiiRoutingDecision::PathMismatch, TiiRoutingDecision::Match(_, _)) => Ordering::Less,
      (TiiRoutingDecision::PathMismatch, TiiRoutingDecision::PathMismatch) => Ordering::Equal,
      (TiiRoutingDecision::PathMismatch, TiiRoutingDecision::MethodMismatch) => Ordering::Less,
      (TiiRoutingDecision::PathMismatch, TiiRoutingDecision::MimeMismatch) => Ordering::Less,
      (TiiRoutingDecision::PathMismatch, TiiRoutingDecision::AcceptMismatch) => Ordering::Less,

      (TiiRoutingDecision::MethodMismatch, TiiRoutingDecision::Match(_, _)) => Ordering::Less,
      (TiiRoutingDecision::MethodMismatch, TiiRoutingDecision::PathMismatch) => Ordering::Greater,
      (TiiRoutingDecision::MethodMismatch, TiiRoutingDecision::MethodMismatch) => Ordering::Equal,
      (TiiRoutingDecision::MethodMismatch, TiiRoutingDecision::MimeMismatch) => Ordering::Less,
      (TiiRoutingDecision::MethodMismatch, TiiRoutingDecision::AcceptMismatch) => Ordering::Less,

      (TiiRoutingDecision::MimeMismatch, TiiRoutingDecision::Match(_, _)) => Ordering::Less,
      (TiiRoutingDecision::MimeMismatch, TiiRoutingDecision::PathMismatch) => Ordering::Greater,
      (TiiRoutingDecision::MimeMismatch, TiiRoutingDecision::MethodMismatch) => Ordering::Greater,
      (TiiRoutingDecision::MimeMismatch, TiiRoutingDecision::MimeMismatch) => Ordering::Equal,
      (TiiRoutingDecision::MimeMismatch, TiiRoutingDecision::AcceptMismatch) => Ordering::Less,

      (TiiRoutingDecision::AcceptMismatch, TiiRoutingDecision::Match(_, _)) => Ordering::Less,
      (TiiRoutingDecision::AcceptMismatch, TiiRoutingDecision::PathMismatch) => Ordering::Greater,
      (TiiRoutingDecision::AcceptMismatch, TiiRoutingDecision::MethodMismatch) => Ordering::Greater,
      (TiiRoutingDecision::AcceptMismatch, TiiRoutingDecision::MimeMismatch) => Ordering::Greater,
      (TiiRoutingDecision::AcceptMismatch, TiiRoutingDecision::AcceptMismatch) => Ordering::Equal,
    }
  }
}

impl TiiRouteable {
  pub(crate) fn new(
    path: impl ToString,
    method: impl Into<TiiHttpMethod>,
    consumes: HashSet<TiiAcceptMimeType>,
    produces: HashSet<TiiAcceptMimeType>,
  ) -> TiiResult<TiiRouteable> {
    let path = path.to_string();
    Ok(TiiRouteable {
      parts: PathPart::parse(path.as_str())?,
      path,
      method: method.into(),
      consumes,
      produces,
    })
  }

  /// The path for this route
  pub fn path(&self) -> &str {
    self.path.as_str()
  }

  /// The method for this route
  pub fn method(&self) -> &TiiHttpMethod {
    &self.method
  }

  /// The mime types this route can consume
  pub fn consumes(&self) -> &HashSet<TiiAcceptMimeType> {
    &self.consumes
  }

  /// The mime types this route can produce
  pub fn produces(&self) -> &HashSet<TiiAcceptMimeType> {
    &self.produces
  }

  fn matches_path(
    &self,
    route: &TiiRequestContext,
    path_params: &mut Option<HashMap<String, String>>,
  ) -> bool {
    let mut request_path = route.request_head().get_path();
    if !request_path.starts_with("/") {
      return false;
    }

    request_path = &request_path[1..];

    if request_path.is_empty() && self.parts.is_empty() {
      return true;
    }

    let mut parts = self.parts.iter();
    loop {
      if let Some((path_part, remaining)) = request_path.split_once("/") {
        if let Some(part) = parts.next() {
          if !part.matches(path_part, request_path, path_params) {
            return false;
          }
          if part.is_tail() {
            return true;
          }

          request_path = remaining;
          continue;
        }

        return false;
      }

      if let Some(part) = parts.next() {
        if !part.matches(request_path, request_path, path_params) {
          return false;
        }

        if part.is_tail() {
          return true;
        }

        request_path = "";
        continue;
      }

      if request_path.is_empty() {
        return true;
      }

      return false;
    }
  }

  /// Checks whether this route matches the given one, respecting its own wildcards only.
  /// For example, `/blog/*` will match `/blog/my-first-post` but not the other way around.
  pub fn matches(&self, route: &TiiRequestContext) -> TiiRoutingDecision {
    let head = route.request_head();
    let mut path_params = None;

    if !self.matches_path(route, &mut path_params) {
      return TiiRoutingDecision::PathMismatch;
    }

    if &self.method != head.get_method() {
      return TiiRoutingDecision::MethodMismatch;
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
        return TiiRoutingDecision::MimeMismatch;
      }
    }

    if self.produces.is_empty() {
      //The endpoint either doesn't produce a body or declares that it will produce a matching body...
      return TiiRoutingDecision::Match(TiiQValue::MAX, path_params);
    }

    let acc = head.get_accept();
    if acc.is_empty() {
      //The client doesn't accept a body.
      return TiiRoutingDecision::MimeMismatch;
    }

    let mut current_q = None;
    for accept in acc {
      for mime in &self.produces {
        if !accept.get_type().permits(mime) {
          continue;
        }

        let qvalue = accept.qvalue();
        if qvalue == TiiQValue::MAX {
          return TiiRoutingDecision::Match(qvalue, path_params);
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
      return TiiRoutingDecision::Match(qval, path_params);
    }

    TiiRoutingDecision::AcceptMismatch
  }
}

impl Debug for HttpRoute {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("HttpRoute({})", self.routeable.path.as_str()))
  }
}

impl Debug for WebSocketRoute {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("HttpRoute({})", self.routeable.path.as_str()))
  }
}

/// Represents a sub-app to run for a specific host.
pub(crate) struct BasicRouter {
  /// This filter/predicate will decide if the router should even serve the request at all
  router_filter: Box<dyn TiiRouterFilter>,

  /// Filters that run before the route is matched.
  /// These filters may modify the path of the request to affect routing decision.
  pre_routing_filters: Vec<Box<dyn TiiRequestFilter>>,

  /// Filters that run once the routing decision has been made.
  /// These filters only run if there is an actual endpoint.
  routing_filters: Vec<Box<dyn TiiRequestFilter>>,

  /// These filters run on the response after the actual endpoint (or the error handler) has been called.
  response_filters: Vec<Box<dyn TiiResponseFilter>>,

  /// Contains all pathing information for websockets and normal http routes.
  /// This is essentially a union of routes and websocket_routes without the handler
  routeables: Vec<TiiRouteable>,

  /// The routes to process requests for and their handlers.
  routes: Vec<HttpRoute>,

  /// The routes to process WebSocket requests for and their handlers.
  websocket_routes: Vec<WebSocketRoute>,

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

impl Debug for BasicRouter {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("TiiRouter(pre_routing_filters={}, routing_filters={}, response_filters={}, routes={:?}, websocket_routes={})",
                                 self.pre_routing_filters.len(),
            self.routing_filters.len(),
            self.response_filters.len(),
            self.routes,
            self.websocket_routes.len(),
        ))
  }
}

/// Performs the WebSocket handshake.
fn websocket_handshake(request: &TiiRequestContext) -> TiiResult<TiiResponse> {
  const HANDSHAKE_KEY_CONSTANT: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

  // Get the handshake key header
  let handshake_key = request
    .request_head()
    .get_header("Sec-WebSocket-Key")
    .ok_or(RequestHeadParsingError::MissingSecWebSocketKeyHeader)?;

  // Calculate the handshake response
  let sha1 =
    Sha1::new().chain_update(handshake_key).chain_update(HANDSHAKE_KEY_CONSTANT).finalize();
  let sec_websocket_accept = base64::prelude::BASE64_STANDARD.encode(sha1);

  //let sec_websocket_accept = sha1.encode();

  // Serialise the handshake response
  let response = TiiResponse::new(TiiStatusCode::SwitchingProtocols)
    .with_header(TiiHttpHeaderName::Upgrade, "websocket")?
    .with_header(TiiHttpHeaderName::Connection, "Upgrade")?
    .with_header("Sec-WebSocket-Accept", sec_websocket_accept)?;

  // Oddly enough I think you can establish a WS connection with a POST request that has data.
  // This will consume that data if it has not already been used by a filter.
  // Some beta versions of Web Sockets used the request body to convey the Sec-WebSocket-Key...
  request.consume_request_body()?;
  Ok(response)
}

impl BasicRouter {
  #[expect(clippy::too_many_arguments)] //Only called by the builder.
  pub(crate) fn new(
    router_filter: Box<dyn TiiRouterFilter>,
    pre_routing_filters: Vec<Box<dyn TiiRequestFilter>>,
    routing_filters: Vec<Box<dyn TiiRequestFilter>>,
    response_filters: Vec<Box<dyn TiiResponseFilter>>,
    routes: Vec<HttpRoute>,
    websocket_routes: Vec<WebSocketRoute>,
    not_found_handler: NotRouteableHandler,
    not_acceptable_handler: NotRouteableHandler,
    method_not_allowed_handler: NotRouteableHandler,
    unsupported_media_type_handler: NotRouteableHandler,
    error_handler: ErrorHandler,
  ) -> Self {
    let mut routeables = Vec::new();
    for x in routes.iter() {
      routeables.push(x.routeable.clone());
    }
    for x in websocket_routes.iter() {
      routeables.push(x.routeable.clone());
    }

    Self {
      router_filter,
      pre_routing_filters,
      routing_filters,
      response_filters,
      routeables,
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
    stream: &dyn TiiConnectionStream,
    request: &mut TiiRequestContext,
  ) -> TiiResult<TiiRouterWebSocketServingResponse> {
    //TODO this fn is too long and has significant duplicate parts with normal http serving.
    //TODO consolidate both impls and split it into smaller sub fn's

    if !self.router_filter.filter(request)? {
      return Ok(TiiRouterWebSocketServingResponse::NotHandled);
    }

    for filter in self.pre_routing_filters.iter() {
      let resp = match filter.filter(request) {
        Ok(Some(res)) => res,
        Ok(None) => continue,
        Err(err) => (self.error_handler)(request, err)?,
      };

      let resp = self.call_response_filters(request, resp)?;
      return Ok(TiiRouterWebSocketServingResponse::HandledWithoutProtocolSwitch(resp));
    }

    let mut best_decision = TiiRoutingDecision::PathMismatch;
    let mut best_handler = None;

    for handler in &self.websocket_routes {
      let decision = handler.routeable.matches(request);
      if best_decision >= decision {
        continue;
      }

      best_decision = decision;
      if let TiiRoutingDecision::Match(qv, _) = &best_decision {
        best_handler = Some(handler);
        if qv == &TiiQValue::MAX {
          break;
        }
      }
    }

    if let Some(handler) = best_handler {
      request.set_routed_path(handler.routeable.path.as_str());
      self.handle_path_parameters(request, &best_decision);

      for filter in self.routing_filters.iter() {
        let resp = match filter.filter(request) {
          Ok(Some(res)) => res,
          Ok(None) => continue,
          Err(err) => (self.error_handler)(request, err)?,
        };

        let resp = self.call_response_filters(request, resp)?;
        return Ok(TiiRouterWebSocketServingResponse::HandledWithoutProtocolSwitch(resp));
      }

      return match websocket_handshake(request) {
        Err(err) => {
          let resp = (self.error_handler)(request, err)?;
          let resp = self.call_response_filters(request, resp)?;
          Ok(TiiRouterWebSocketServingResponse::HandledWithoutProtocolSwitch(resp))
        }
        Ok(resp) => {
          let resp = self.call_response_filters(request, resp)?;
          if resp.status_code != TiiStatusCode::SwitchingProtocols {
            return Ok(TiiRouterWebSocketServingResponse::HandledWithoutProtocolSwitch(resp));
          }

          resp.write_to(TiiHttpVersion::Http11, stream)?; //Errors here are fatal

          let (sender, receiver) = crate::new_web_socket_stream(stream);
          handler.handler.serve(request, receiver, sender)?;
          Ok(TiiRouterWebSocketServingResponse::HandledWithProtocolSwitch)
        }
      };
    }

    trace_log!("WebsocketConnectionClosed Invoke fallback {}", &best_decision);

    let fallback = self.invoke_appropriate_fallback_handler(request, &best_decision);

    let fallback_resp = match fallback {
      Ok(resp) => self.call_response_filters(request, resp)?,
      Err(err) => {
        let resp = (self.error_handler)(request, err)?;
        self.call_response_filters(request, resp)?
      }
    };
    Ok(TiiRouterWebSocketServingResponse::HandledWithoutProtocolSwitch(fallback_resp))
  }

  fn handle_path_parameters(
    &self,
    request: &mut TiiRequestContext,
    best_decision: &TiiRoutingDecision,
  ) {
    match best_decision {
      TiiRoutingDecision::Match(_, path_params) => {
        if let Some(path_params) = path_params {
          for (key, value) in path_params {
            request.set_path_param(key, value);
          }
        }
      }
      _ => util::unreachable(),
    }
  }

  fn call_error_handler(
    &self,
    request: &mut TiiRequestContext,
    error: TiiError,
  ) -> TiiResult<TiiResponse> {
    //TODO i am not 100% sure this is a good idea, but it probably is a good idea.
    //The only thing i could consider is having the default impl do this and outsource this responsibility to the user
    //Not doing this on io::Errors when reading the request body will cause stuff to break in a horrific manner.
    //So always doing this is safer, but prevents keepalive in cases where the error is unrelated to the http stream
    //and the user properly handles it.
    request.force_connection_close();

    (self.error_handler)(request, error)
  }

  fn serve_outer(&self, request: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>> {
    if !self.router_filter.filter(request)? {
      return Ok(None);
    }

    let mut resp = self.serve_inner(request).or_else(|e| self.call_error_handler(request, e))?;
    resp = self.call_response_filters(request, resp)?;

    Ok(Some(resp))
  }

  fn call_response_filters(
    &self,
    request: &mut TiiRequestContext,
    mut resp: TiiResponse,
  ) -> TiiResult<TiiResponse> {
    for filter in self.response_filters.iter() {
      resp = filter.filter(request, resp).or_else(|e| self.call_error_handler(request, e))?;
    }
    Ok(resp)
  }

  fn serve_inner(&self, request: &mut TiiRequestContext) -> TiiResult<TiiResponse> {
    for filter in self.pre_routing_filters.iter() {
      if let Some(resp) = filter.filter(request)? {
        return Ok(resp);
      }
    }

    let mut best_decision = TiiRoutingDecision::PathMismatch;
    let mut best_handler = None;

    for handler in &self.routes {
      let decision = handler.routeable.matches(request);
      if best_decision >= decision {
        continue;
      }

      best_decision = decision;
      if let TiiRoutingDecision::Match(qv, _) = &best_decision {
        best_handler = Some(handler);
        if qv == &TiiQValue::MAX {
          break;
        }
      }
    }

    if let Some(handler) = best_handler {
      request.set_routed_path(handler.routeable.path.as_str());
      self.handle_path_parameters(request, &best_decision);

      for filter in self.routing_filters.iter() {
        if let Some(resp) = filter.filter(request)? {
          return Ok(resp);
        }
      }

      return handler.handler.serve(request);
    }

    self.invoke_appropriate_fallback_handler(request, &best_decision)
  }

  fn invoke_appropriate_fallback_handler(
    &self,
    request: &mut TiiRequestContext,
    best_decision: &TiiRoutingDecision,
  ) -> TiiResult<TiiResponse> {
    match best_decision {
      TiiRoutingDecision::PathMismatch => (self.not_found_handler)(request, &self.routeables),
      TiiRoutingDecision::MethodMismatch => {
        (self.method_not_allowed_handler)(request, &self.routeables)
      }
      TiiRoutingDecision::MimeMismatch => {
        (self.unsupported_media_type_handler)(request, &self.routeables)
      }
      TiiRoutingDecision::AcceptMismatch => {
        (self.not_acceptable_handler)(request, &self.routeables)
      }
      // We found a handler! Why are we here?
      TiiRoutingDecision::Match(_, _) => util::unreachable(),
    }
  }
}

impl TiiRouter for BasicRouter {
  fn serve(&self, request: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>> {
    self.serve_outer(request)
  }

  fn serve_websocket(
    &self,
    stream: &dyn TiiConnectionStream,
    request: &mut TiiRequestContext,
  ) -> TiiResult<TiiRouterWebSocketServingResponse> {
    self.serve_ws(stream, request)
  }
}

impl TiiRouter for Arc<BasicRouter> {
  fn serve(&self, request: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>> {
    Arc::as_ref(self).serve(request)
  }

  fn serve_websocket(
    &self,
    stream: &dyn TiiConnectionStream,
    request: &mut TiiRequestContext,
  ) -> TiiResult<TiiRouterWebSocketServingResponse> {
    Arc::as_ref(self).serve_websocket(stream, request)
  }
}
