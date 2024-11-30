//! Defines traits for handler and filter functions.

use crate::http::request_context::RequestContext;
use crate::http::Response;
use crate::humpty_error::HumptyResult;
use crate::stream::ConnectionStream;
use crate::websocket::stream::{WebsocketReceiver, WebsocketSender};
use std::fmt::{Debug, Formatter};
use std::thread;

/// Represents an opaque join handle
pub struct ThreadAdapterJoinHandle(pub Box<dyn FnOnce() -> thread::Result<()> + Send>);

impl ThreadAdapterJoinHandle {
  /// Calls the join fn
  pub fn join(self) -> thread::Result<()> {
    self.0()
  }
}

impl Debug for ThreadAdapterJoinHandle {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("ThreadAdapterJoinHandle")
  }
}

impl Default for ThreadAdapterJoinHandle {
  fn default() -> Self {
    Self(Box::new(|| Ok(())))
  }
}

/// Trait that represents a user implemented opaque thread starting/pooling mechanism.
pub trait ThreadAdapter: Send + Sync {
  /// Spawns executes the given task immediately in the thread. like "thread::spawn".
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> HumptyResult<ThreadAdapterJoinHandle>;
}

#[allow(dead_code)] //This is not used in all feature combinations.
pub(crate) struct DefaultThreadAdapter;
impl ThreadAdapter for DefaultThreadAdapter {
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> HumptyResult<ThreadAdapterJoinHandle> {
    let hdl = thread::Builder::new().spawn(task)?;
    Ok(ThreadAdapterJoinHandle(Box::new(move || hdl.join())))
  }
}

/// Represents a function able to handle a WebSocket handshake and consequent data frames.
pub trait WebsocketEndpoint: Send + Sync {
  /// serve the web socket request.
  fn serve(
    &self,
    request: &RequestContext,
    receiver: WebsocketReceiver,
    sender: WebsocketSender,
  ) -> HumptyResult<()>;
}

trait IntoWebsocketEndpointResponse {
  fn into(self) -> HumptyResult<()>;
}

impl IntoWebsocketEndpointResponse for HumptyResult<()> {
  fn into(self) -> HumptyResult<()> {
    self
  }
}

impl IntoWebsocketEndpointResponse for () {
  fn into(self) -> HumptyResult<()> {
    Ok(())
  }
}

impl<F, R> WebsocketEndpoint for F
where
  R: IntoWebsocketEndpointResponse,
  F: Fn(&RequestContext, WebsocketReceiver, WebsocketSender) -> R + Send + Sync,
{
  fn serve(
    &self,
    request: &RequestContext,
    receiver: WebsocketReceiver,
    sender: WebsocketSender,
  ) -> HumptyResult<()> {
    self(request, receiver, sender).into()
  }
}

/// Represents a function able to handle a request.
/// It is passed the request and must return a response.
///
/// ## Example
/// The most basic request handler would be as follows:
/// ```
/// use humpty::http::mime::MimeType;
///
/// fn handler(_: humpty::http::RequestHead) -> humpty::http::Response {
///     humpty::http::Response::ok("Success", MimeType::TextPlain)
/// }
/// ```
pub trait HttpEndpoint: Send + Sync {
  /// Serve an ordinary http request.
  fn serve(&self, request: &RequestContext) -> HumptyResult<Response>;
}

impl<F, R> HttpEndpoint for F
where
  R: Into<HumptyResult<Response>>,
  F: Fn(&RequestContext) -> R + Send + Sync,
{
  fn serve(&self, request: &RequestContext) -> HumptyResult<Response> {
    self(request).into()
  }
}

/// Trait for a "filter" that decide if a router is responsible for handling a request.
/// Intended use is to do matching on things like base path, Host HTTP Header,
/// some other magic header.
pub trait RouterFilter: Send + Sync {
  /// true -> the router should handle this one,
  /// false -> the router should not handle this one,
  //TODO make it impossible for this shit to read the body.
  fn filter(&self, request: &RequestContext) -> HumptyResult<bool>;
}

impl<F: Fn(&RequestContext) -> HumptyResult<bool> + Send + Sync> RouterFilter for F {
  fn filter(&self, request: &RequestContext) -> HumptyResult<bool> {
    self(request)
  }
}

trait IntoRequestFilterResult {
  fn into(self) -> HumptyResult<Option<Response>>;
}

impl IntoRequestFilterResult for Option<Response> {
  fn into(self) -> HumptyResult<Option<Response>> {
    Ok(self)
  }
}

impl IntoRequestFilterResult for HumptyResult<Option<Response>> {
  fn into(self) -> HumptyResult<Option<Response>> {
    self
  }
}
impl IntoRequestFilterResult for () {
  fn into(self) -> HumptyResult<Option<Response>> {
    Ok(None)
  }
}

impl IntoRequestFilterResult for HumptyResult<()> {
  fn into(self) -> HumptyResult<Option<Response>> {
    self.map(|_| None)
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
  fn filter(&self, request: &mut RequestContext) -> HumptyResult<Option<Response>>;
}

impl<F, R> RequestFilter for F
where
  R: IntoRequestFilterResult,
  F: Fn(&mut RequestContext) -> R + Send + Sync,
{
  fn filter(&self, request: &mut RequestContext) -> HumptyResult<Option<Response>> {
    self(request).into()
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
  fn filter(&self, request: &mut RequestContext, response: Response) -> HumptyResult<Response>;
}

impl<F, R> ResponseFilter for F
where
  R: Into<HumptyResult<Response>>,
  F: Fn(&mut RequestContext, Response) -> R + Send + Sync,
{
  fn filter(&self, request: &mut RequestContext, response: Response) -> HumptyResult<Response> {
    self(request, response).into()
  }
}

/// A router may respond to a web-socket request with a http response or indicate that the socket has been handled with a protocol switch
/// Or it may indicate that it hasn't handled the socket and signal that the next router should do it.
/// This enum represents those 3 states
#[derive(Debug)]
pub enum RouterWebSocketServingResponse {
  /// Handled with protocol switch to WS
  HandledWithProtocolSwitch,
  /// Handled using HTTP protocol
  HandledWithoutProtocolSwitch(Response),
  /// Not handled, next router should do it.
  NotHandled,
}

/// Trait for a router.
pub trait Router: Debug + Send + Sync {
  /// Handle an ordinary http request
  /// Ok(Some) -> request was handled
  /// Ok(None) -> request was not handled and should be handled by the next router
  /// Err -> abort
  ///
  /// Note: If the request body is read then returning Ok(None) will most likely result in unintended behavior in the next Router.
  fn serve(&self, request: &mut RequestContext) -> HumptyResult<Option<Response>>;

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
  ) -> HumptyResult<RouterWebSocketServingResponse>;
}
