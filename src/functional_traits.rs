//! Defines traits for handler and filter functions.

use crate::TiiConnectionStream;
use crate::TiiRequestContext;
use crate::TiiResponse;
use crate::TiiResult;
use crate::{TiiWebsocketReceiver, TiiWebsocketSender};
use std::fmt::{Debug, Formatter};
use std::thread;
use std::thread::JoinHandle;

/// Represents an opaque join handle
pub struct TiiThreadAdapterJoinHandle(Box<dyn FnOnce() -> thread::Result<()> + Send>);

impl TiiThreadAdapterJoinHandle {
  /// Constructor
  pub fn new(inner: Box<dyn FnOnce() -> thread::Result<()> + Send>) -> Self {
    TiiThreadAdapterJoinHandle(inner)
  }

  /// Calls the join fn
  pub fn join(self) -> thread::Result<()> {
    self.0()
  }
}

impl Debug for TiiThreadAdapterJoinHandle {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("ThreadAdapterJoinHandle")
  }
}

impl Default for TiiThreadAdapterJoinHandle {
  fn default() -> Self {
    Self(Box::new(|| Ok(())))
  }
}

/// Trait that represents a user implemented opaque thread starting/pooling mechanism.
pub trait TiiThreadAdapter: Send + Sync + Debug {
  /// Spawns executes the given task immediately in the thread. like "thread::spawn".
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> TiiResult<TiiThreadAdapterJoinHandle>;
}

#[allow(dead_code)] //This is not used in all feature combinations.
#[derive(Debug)]
pub(crate) struct DefaultThreadAdapter;
impl TiiThreadAdapter for DefaultThreadAdapter {
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> TiiResult<TiiThreadAdapterJoinHandle> {
    let hdl: JoinHandle<()> = thread::Builder::new().spawn(task)?;
    Ok(TiiThreadAdapterJoinHandle::new(Box::new(move || hdl.join())))
  }
}

/// Represents a function able to handle a WebSocket handshake and consequent data frames.
pub trait TiiWebsocketEndpoint: Send + Sync {
  /// serve the web socket request.
  fn serve(
    &self,
    request: &TiiRequestContext,
    receiver: TiiWebsocketReceiver,
    sender: TiiWebsocketSender,
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

impl<F, R> TiiWebsocketEndpoint for F
where
  R: IntoWebsocketEndpointResponse,
  F: Fn(&TiiRequestContext, TiiWebsocketReceiver, TiiWebsocketSender) -> R + Send + Sync,
{
  fn serve(
    &self,
    request: &TiiRequestContext,
    receiver: TiiWebsocketReceiver,
    sender: TiiWebsocketSender,
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
/// use tii::TiiMimeType;
///
/// fn handler(_: tii::TiiRequestHead) -> tii::TiiResponse {
///     tii::TiiResponse::ok("Success", TiiMimeType::TextPlain)
/// }
/// ```
pub trait TiiHttpEndpoint: Send + Sync {
  /// Serve an ordinary http request.
  fn serve(&self, request: &TiiRequestContext) -> TiiResult<TiiResponse>;
}

impl<F, R> TiiHttpEndpoint for F
where
  R: Into<TiiResult<TiiResponse>>,
  F: Fn(&TiiRequestContext) -> R + Send + Sync,
{
  fn serve(&self, request: &TiiRequestContext) -> TiiResult<TiiResponse> {
    self(request).into()
  }
}

/// Trait for a "filter" that decide if a router is responsible for handling a request.
/// Intended use is to do matching on things like base path, Host HTTP Header,
/// some other magic header.
pub trait TiiRouterFilter: Send + Sync {
  /// true -> the router should handle this one,
  /// false -> the router should not handle this one,
  //TODO make it impossible for this shit to read the body.
  fn filter(&self, request: &TiiRequestContext) -> TiiResult<bool>;
}

impl<F: Fn(&TiiRequestContext) -> TiiResult<bool> + Send + Sync> TiiRouterFilter for F {
  fn filter(&self, request: &TiiRequestContext) -> TiiResult<bool> {
    self(request)
  }
}

trait IntoTiiRequestFilterResult {
  fn into(self) -> TiiResult<Option<TiiResponse>>;
}

impl IntoTiiRequestFilterResult for Option<TiiResponse> {
  fn into(self) -> TiiResult<Option<TiiResponse>> {
    Ok(self)
  }
}

impl IntoTiiRequestFilterResult for TiiResult<Option<TiiResponse>> {
  fn into(self) -> TiiResult<Option<TiiResponse>> {
    self
  }
}
impl IntoTiiRequestFilterResult for () {
  fn into(self) -> TiiResult<Option<TiiResponse>> {
    Ok(None)
  }
}

impl IntoTiiRequestFilterResult for TiiResult<()> {
  fn into(self) -> TiiResult<Option<TiiResponse>> {
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
pub trait TiiRequestFilter: Send + Sync {
  /// Called with the request context before the endpoint is called.
  /// Ok(None) -> proceed.
  /// Ok(Some) -> abort request with given response.
  /// Err -> Call error handler and proceed (endpoint won't be called)
  fn filter(&self, request: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>>;
}

impl<F, R> TiiRequestFilter for F
where
  R: IntoTiiRequestFilterResult,
  F: Fn(&mut TiiRequestContext) -> R + Send + Sync,
{
  fn filter(&self, request: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>> {
    self(request).into()
  }
}

/// Trait for a filter that may alter a Response after an endpoint has been called or a filter has aborted the request.
/// Use cases: (Non-Exhaustive)
/// - Adding Cors headers
/// - Adding Various other headers
/// - Logging of the response
/// - "Rough" estimation of the time it takes for the endpoint to process things.
pub trait TiiResponseFilter: Send + Sync {
  /// Called with the request context adn response after the endpoint or error handler is called.
  /// Ok(...) -> proceed.
  /// Err -> Call error handler and proceed. (You cannot create a loop, a Response filter will only be called exactly once per RequestContext)
  fn filter(
    &self,
    request: &mut TiiRequestContext,
    response: TiiResponse,
  ) -> TiiResult<TiiResponse>;
}

impl<F, R> TiiResponseFilter for F
where
  R: Into<TiiResult<TiiResponse>>,
  F: Fn(&mut TiiRequestContext, TiiResponse) -> R + Send + Sync,
{
  fn filter(
    &self,
    request: &mut TiiRequestContext,
    response: TiiResponse,
  ) -> TiiResult<TiiResponse> {
    self(request, response).into()
  }
}

/// A router may respond to a web-socket request with a http response or indicate that the socket has been handled with a protocol switch
/// Or it may indicate that it hasn't handled the socket and signal that the next router should do it.
/// This enum represents those 3 states
#[derive(Debug)]
pub enum TiiRouterWebSocketServingResponse {
  /// Handled with protocol switch to WS
  HandledWithProtocolSwitch,
  /// Handled using HTTP protocol
  HandledWithoutProtocolSwitch(TiiResponse),
  /// Not handled, next router should do it.
  NotHandled,
}

/// Trait for a router.
pub trait TiiRouter: Debug + Send + Sync {
  /// Handle an ordinary http request
  /// Ok(Some) -> request was handled
  /// Ok(None) -> request was not handled and should be handled by the next router
  /// Err -> abort
  ///
  /// Note: If the request body is read then returning Ok(None) will most likely result in unintended behavior in the next Router.
  fn serve(&self, request: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>>;

  /// Handle a web socket request.
  /// Ok(true) -> request was handled
  /// Ok(false) -> request should not be handled by this router
  /// Err -> abort
  ///
  /// Note: If the stream is read or written to then returning Ok(false) will most likely result in unintended behavior in the next Router.
  fn serve_websocket(
    &self,
    stream: &dyn TiiConnectionStream,
    request: &mut TiiRequestContext,
  ) -> TiiResult<TiiRouterWebSocketServingResponse>;
}
