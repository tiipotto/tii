//! Server impl takes care of reading the RequestHead, deciding what type of body there is and delegating processing to a router.
//! It also handles http keep alive and rudimentary (fallback) error handling.
//! If no router wants to handle the request it also has a 404 handler.

use crate::functional_traits::Router;
use crate::http::headers::HeaderName;
use crate::http::request::HttpVersion;
use crate::http::request_context::RequestContext;
use crate::http::{Response, StatusCode};
use crate::humpty_builder::{ErrorHandler, NotFoundHandler};
use crate::humpty_error::{HumptyError, HumptyResult};
use crate::stream::IntoConnectionStream;
use crate::{error_log, trace_log};
use std::any::Any;
use std::fmt::Debug;
use std::io;
use std::sync::Arc;
use std::time::Duration;

/// Trait for metadata for streams. This could for example be an indicator of what type of stream this is
/// if this is relevant for your application. For example an app may ingest connections from a plain and tls socket at the same time.
/// This could be used to indicate this, or indicate things like "is there a client Certficate present and if so which?"
/// This type is opaque intentionally, and it is left entirely up to the user if such metadata is needed and present when
/// processing a stream.
pub trait ConnectionStreamMetadata: Any + Debug + Send + Sync {
  /// upcast to dyn Any. most likely just return "self".
  fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
struct PhantomStreamMetadata;

impl ConnectionStreamMetadata for PhantomStreamMetadata {
  fn as_any(&self) -> &dyn Any {
    // This type is never instantiated. therefore this is unreachable.
    unreachable!()
  }
}

/// Struct that represents a built server capable of handling connections from some sources.
/// It does NOT own any OS resources like server sockets / file descriptors.
#[derive(Debug)]
pub struct HumptyServer {
  error_handler: ErrorHandler,
  not_found_handler: NotFoundHandler,
  timeout: Option<Duration>,
  routers: Vec<Box<dyn Router>>,
}
impl HumptyServer {
  pub(crate) fn new(
    sub_apps: Vec<Box<dyn Router>>,
    error_handler: ErrorHandler,
    not_found_handler: NotFoundHandler,
    timeout: Option<Duration>,
  ) -> Self {
    HumptyServer { error_handler, not_found_handler, timeout, routers: sub_apps }
  }

  /// Handles a connection without any metadata
  pub fn handle_connection<S: IntoConnectionStream>(&self, stream: S) -> HumptyResult<()> {
    self.handle_connection_inner::<S, PhantomStreamMetadata>(stream, None)
  }

  /// Handles a connection with arbitrary metadata
  pub fn handle_connection_with_meta<S: IntoConnectionStream, M: ConnectionStreamMetadata>(
    &self,
    stream: S,
    meta: M,
  ) -> HumptyResult<()> {
    self.handle_connection_inner(stream, Some(meta))
  }

  /// Impl for handle connection.
  fn handle_connection_inner<S: IntoConnectionStream, M: ConnectionStreamMetadata>(
    &self,
    stream: S,
    meta: Option<M>,
  ) -> HumptyResult<()> {
    let stream = stream.into_connection_stream();
    //TODO split this into 2 parameters? Or make multiple parameters for different stages.
    //Use may desire timeout for request header but LOOOOONG/Infinite timeout for endpoints?
    //I am not a fan of exposing this method to the endpoints... but this may be a good idea anyways...
    stream.set_read_timeout(self.timeout)?;
    stream.set_write_timeout(self.timeout)?;

    let meta = meta.map(|a| Arc::new(a) as Arc<dyn ConnectionStreamMetadata>);

    loop {
      let mut context = RequestContext::new(stream.as_ref(), meta.as_ref().cloned())?;

      // If the request is valid an is a WebSocket request, call the corresponding handler
      if context.request_head().version == HttpVersion::Http11
        && context.request_head().headers.get(&HeaderName::Upgrade) == Some("websocket")
      {
        //Http 1.0 or 0.9 does not have web sockets

        trace_log!("WebsocketConnectionRequested");

        for router in self.routers.iter() {
          if router.serve_websocket(stream.as_ref(), &mut context)? {
            return Ok(());
          }
        }

        // TODO how can I tell a websocket request gracefully that there is no one here for it? HTTP 404?, this just shuts the socket.
        trace_log!("WebsocketConnectionClosed Not found");
        return Ok(());
      }

      // Is the keep alive header set?
      let mut keep_alive = context.request_head().version == HttpVersion::Http11
        && context
          .request_head()
          .headers
          .get(&HeaderName::Connection)
          .map(|e| e.eq_ignore_ascii_case("keep-alive"))
          .unwrap_or_default();

      let mut response = None;
      for router in self.routers.iter() {
        response = Some(match router.serve(&mut context) {
          Ok(Some(resp)) => resp,
          Ok(None) => continue,
          Err(error) => (self.error_handler)(&mut context, error)
            .unwrap_or_else(|e| self.fallback_error_handler(&mut context, e)),
        });

        break;
      }

      let mut response = response.unwrap_or_else(|| match (self.not_found_handler)(&mut context) {
        Ok(res) => res,
        Err(error) => (self.error_handler)(&mut context, error)
          .unwrap_or_else(|e| self.fallback_error_handler(&mut context, e)),
      });

      context.consume_request_body()?;

      keep_alive &= !context.is_connection_close_forced();

      if context.request_head().version == HttpVersion::Http11 {
        let previous_headers = if keep_alive {
          response.headers.replace_all(HeaderName::Connection, "Keep-Alive")
        } else {
          response.headers.replace_all(HeaderName::Connection, "Close")
        };

        if !previous_headers.is_empty() {
          trace_log!("Endpoint has set banned header 'Connection' {:?}", previous_headers);
          return Err(HumptyError::new_io(
            io::ErrorKind::InvalidInput,
            "Endpoint has set banned header 'Connection'",
          ));
        }
      }

      trace_log!("RequestRespondedWith HTTP {}", response.status_code.code());

      response.write_to(context.request_head().version, stream.as_stream_write()).inspect_err(
        |e| {
          trace_log!("response.write_to {}", e);
        },
      )?;

      trace_log!("RequestServedSuccess");

      // If the request specified to keep the connection open, respect this
      if !keep_alive {
        trace_log!("NoKeepAlive");
        break;
      }

      trace_log!("KeepAliveRespected");
    }

    trace_log!("ConnectionClosed");
    Ok(())
  }

  fn fallback_error_handler(&self, request: &mut RequestContext, error: HumptyError) -> Response {
    request.force_connection_close();

    error_log!(
      "Error handler failed. Will respond with empty Internal Server Error {} {} {:?}",
      &request.request_head().method,
      request.request_head().path.as_str(),
      error
    );

    Response::empty(StatusCode::InternalServerError)
  }
}
