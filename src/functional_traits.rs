//! Defines traits for handler and filter functions.

use crate::http::request_context::RequestContext;
use crate::http::Response;
use crate::tii_error::TiiResult;
use crate::stream::ConnectionStream;
use crate::websocket::stream::{WebsocketReceiver, WebsocketSender};
use std::fmt::{Debug, Formatter};
use std::thread;
use std::thread::JoinHandle;

/// Represents an opaque join handle
pub struct ThreadAdapterJoinHandle(Box<dyn FnOnce() -> thread::Result<()> + Send>);

impl ThreadAdapterJoinHandle {
  /// Constructor
  pub fn new(inner: Box<dyn FnOnce() -> thread::Result<()> + Send>) -> Self {
    ThreadAdapterJoinHandle(inner)
  }

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
pub trait ThreadAdapter: Send + Sync + Debug {
  /// Spawns executes the given task immediately in the thread. like "thread::spawn".
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> TiiResult<ThreadAdapterJoinHandle>;
}

#[allow(dead_code)] //This is not used in all feature combinations.
#[derive(Debug)]
pub(crate) struct DefaultThreadAdapter;
impl ThreadAdapter for DefaultThreadAdapter {
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> TiiResult<ThreadAdapterJoinHandle> {
    let hdl: JoinHandle<()> = thread::Builder::new().spawn(task)?;
    Ok(ThreadAdapterJoinHandle::new(Box::new(move || hdl.join())))
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
  ) -> TiiResult<()>;
}

trait IntoWebsocketEndpointResponse {
  fn into(self) -> TiiResult<()>;
}

impl IntoWebsocketEndpointResponse for TiiResult<()> {
  fn into(self) -> TiiResult<()> {
    self
  }
}

impl IntoWebsocketEndpointResponse for () {
  fn into(self) -> TiiResult<()> {
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
  ) -> TiiResult<()> {
    self(request, receiver, sender).into()
  }
}

/// Represents a function able to handle a request.
/// It is passed the request and must return a response.
///
/// ## Example
/// The most basic request handler would be as follows:
/// ```
/// use tii::http::mime::MimeType;
///
/// fn handler(_: tii::http::RequestHead) -> tii::http::Response {
///     tii::http::Response::ok("Success", MimeType::TextPlain)
/// }
/// ```
pub trait HttpEndpoint: Send + Sync {
  /// Serve an ordinary http request.
  fn serve(&self, request: &RequestContext) -> TiiResult<Response>;
}

impl<F, R> HttpEndpoint for F
where
  R: Into<TiiResult<Response>>,
  F: Fn(&RequestContext) -> R + Send + Sync,
{
  fn serve(&self, request: &RequestContext) -> TiiResult<Response> {
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
  fn filter(&self, request: &RequestContext) -> TiiResult<bool>;
}

impl<F: Fn(&RequestContext) -> TiiResult<bool> + Send + Sync> RouterFilter for F {
  fn filter(&self, request: &RequestContext) -> TiiResult<bool> {
    self(request)
  }
}

trait IntoRequestFilterResult {
  fn into(self) -> TiiResult<Option<Response>>;
}

impl IntoRequestFilterResult for Option<Response> {
  fn into(self) -> TiiResult<Option<Response>> {
    Ok(self)
  }
}

impl IntoRequestFilterResult for TiiResult<Option<Response>> {
  fn into(self) -> TiiResult<Option<Response>> {
    self
  }
}
impl IntoRequestFilterResult for () {
  fn into(self) -> TiiResult<Option<Response>> {
    Ok(None)
  }
}

impl IntoRequestFilterResult for TiiResult<()> {
  fn into(self) -> TiiResult<Option<Response>> {
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
  fn filter(&self, request: &mut RequestContext) -> TiiResult<Option<Response>>;
}

impl<F, R> RequestFilter for F
where
  R: IntoRequestFilterResult,
  F: Fn(&mut RequestContext) -> R + Send + Sync,
{
  fn filter(&self, request: &mut RequestContext) -> TiiResult<Option<Response>> {
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
  fn filter(&self, request: &mut RequestContext, response: Response) -> TiiResult<Response>;
}

impl<F, R> ResponseFilter for F
where
  R: Into<TiiResult<Response>>,
  F: Fn(&mut RequestContext, Response) -> R + Send + Sync,
{
  fn filter(&self, request: &mut RequestContext, response: Response) -> TiiResult<Response> {
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
  fn serve(&self, request: &mut RequestContext) -> TiiResult<Option<Response>>;

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
  ) -> TiiResult<RouterWebSocketServingResponse>;
}
