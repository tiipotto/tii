//! Provides a Humpty-compatible WebSocket handler for performing the handshake.

use crate::websocket::stream::WebsocketStream;
use crate::websocket::MAGIC_STRING;
use base64::Engine;
use sha1::{Digest, Sha1};

use crate::http::headers::HeaderName;
use crate::http::{RequestHead, Response, StatusCode};
use crate::humpty_error::{HumptyResult, WebsocketError};
use crate::stream::ConnectionStream;

/// Represents a function able to handle WebSocket streams.
pub trait WebsocketHandler: Fn(WebsocketStream) + Send + Sync {}
impl<T> WebsocketHandler for T where T: Fn(WebsocketStream) + Send + Sync {}

/// Provides WebSocket handshake functionality.
/// Supply a `WebsocketHandler` to handle the subsequent messages.
///
pub fn websocket_handler<T>(handler: T) -> impl Fn(RequestHead, Box<dyn ConnectionStream>)
where
  T: WebsocketHandler,
{
  move |request: RequestHead, mut stream: Box<dyn ConnectionStream>| {
    if handshake(request, &mut stream).is_ok() {
      handler(WebsocketStream::new(stream));
    }
  }
}

/// Performs the WebSocket handshake.
fn handshake(request: RequestHead, stream: &mut Box<dyn ConnectionStream>) -> HumptyResult<()> {
  // Get the handshake key header
  let handshake_key =
    request.get_header("Sec-WebSocket-Key").ok_or(WebsocketError::HandshakeError)?;

  // Calculate the handshake response
  let sha1 = Sha1::new().chain_update(format!("{}{}", handshake_key, MAGIC_STRING)).finalize();
  let sec_websocket_accept = base64::prelude::BASE64_STANDARD.encode(sha1);

  //let sec_websocket_accept = sha1.encode();

  // Serialise the handshake response
  let response = Response::new(StatusCode::SwitchingProtocols)
    .with_header(HeaderName::Upgrade, "websocket")
    .map_err(|_| WebsocketError::HandshakeError)?
    .with_header(HeaderName::Connection, "Upgrade")
    .map_err(|_| WebsocketError::HandshakeError)?
    .with_header("Sec-WebSocket-Accept", sec_websocket_accept)
    .map_err(|_| WebsocketError::HandshakeError)?;

  // Transmit the handshake response
  response.write_to(request.version(), stream.as_stream_write())?;

  Ok(())
}
