//! Defines traits for handler and filter functions.

use crate::http::request_context::RequestContext;
use crate::http::{RequestHead, Response};
use crate::stream::ConnectionStream;
use std::fmt::Debug;
use std::io;

/// Represents a function able to handle a WebSocket handshake and consequent data frames.
pub trait WebsocketHandler: Send + Sync {
  /// serve the web socket request.
  fn serve(&self, request: RequestHead, stream: Box<dyn ConnectionStream>);
}
impl<F> WebsocketHandler for F
where
  F: Fn(RequestHead, Box<dyn ConnectionStream>) + Send + Sync,
{
  fn serve(&self, request: RequestHead, stream: Box<dyn ConnectionStream>) {
    self(request, stream)
  }
}

/// Represents a function able to handle a request.
/// It is passed the request and must return a response.
///
/// ## Example
/// The most basic request handler would be as follows:
/// ```
/// fn handler(_: humpty::http::RequestHead) -> humpty::http::Response {
///     humpty::http::Response::new(humpty::http::StatusCode::OK, b"Success")
/// }
/// ```
pub trait RequestHandler: Send + Sync {
  /// Serve an ordinary http request.
  fn serve(&self, request: &RequestContext) -> io::Result<Response>;
}

impl<F> RequestHandler for F
where
  F: Fn(&RequestContext) -> io::Result<Response> + Send + Sync,
{
  fn serve(&self, request: &RequestContext) -> io::Result<Response> {
    self(request)
  }
}

/// Trait for a "filter" that decide if a router is responsible for handling a request.
/// Intended use is to do matching on things like base path, Host HTTP Header,
/// some other magic header.
pub trait RouterFilter: Send + Sync {
  /// true -> the router should handle this one,
  /// false -> the router should not handle this one,
  //TODO make it impossible for this shit to read the body.
  fn filter(&self, request: &RequestContext) -> io::Result<bool>;
}

impl<F: Fn(&RequestContext) -> io::Result<bool> + Send + Sync> RouterFilter for F {
  fn filter(&self, request: &RequestContext) -> io::Result<bool> {
    self(request)
  }
}

/// Trait for a filter that may alter a request before its brought to an endpoint.
/// It's also capable of aborting a request so that it's not processed further.
/// Use cases: (Non-Exhaustive)
/// - Authentication/Authorization
/// - Transforming of the request entity. (I.e. transform json)
/// - Logging of the request
/// - "Rough" estimation of the time it takes for the endpoint to process things.
pub trait RequestFilter: Send + Sync {
  /// Called with the request context before the endpoint is called.
  /// Ok(None) -> proceed.
  /// Ok(Some) -> abort request with given response.
  /// Err -> Call error handler and proceed (endpoint won't be called)
  fn filter(&self, request: &mut RequestContext) -> io::Result<Option<Response>>;
}

impl<F: Fn(&mut RequestContext) -> io::Result<Option<Response>> + Send + Sync> RequestFilter for F {
  fn filter(&self, request: &mut RequestContext) -> io::Result<Option<Response>> {
    self(request)
  }
}

/// Trait for a filter that may alter a Response after an endpoint has been called or a filter has aborted the request.
/// Use cases: (Non-Exhaustive)
/// - Adding Cors headers
/// - Adding Various other headers
/// - Logging of the response
/// - "Rough" estimation of the time it takes for the endpoint to process things.
pub trait ResponseFilter: Send + Sync {
  /// Called with the request context adn response after the endpoint or error handler is called.
  /// Ok(...) -> proceed.
  /// Err -> Call error handler and proceed. (You cannot create a loop, a Response filter will only be called exactly once per RequestContext)
  fn filter(&self, request: &mut RequestContext, response: Response) -> io::Result<Response>;
}

impl<F: Fn(&mut RequestContext, Response) -> io::Result<Response> + Send + Sync> ResponseFilter
  for F
{
  fn filter(&self, request: &mut RequestContext, response: Response) -> io::Result<Response> {
    self(request, response)
  }
}

/// Trait for a router.
pub trait Router: Debug + Send + Sync {
  /// Handle an ordinary http request
  /// Ok(Some) -> request was handled
  /// Ok(None) -> request was not handled and should be handled by the next router
  /// Err -> abort
  ///
  /// Note: If the request body is read then returning Ok(None) will most likely result in unintended behavior in the next Router.
  fn serve(&self, request: &mut RequestContext) -> io::Result<Option<Response>>;

  /// Handle a web socket request.
  /// Ok(true) -> request was handled
  /// Ok(false) -> request should not be handled by this router
  /// Err -> abort
  ///
  /// Note: If the stream is read or written to then returning Ok(false) will most likely result in unintended behavior in the next Router.
  fn serve_websocket(
    &self,
    stream: &dyn ConnectionStream,
    request: &mut RequestContext,
  ) -> io::Result<bool>;
}
