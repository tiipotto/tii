//! Provides a Humpty-compatible WebSocket handler for performing the handshake.

use crate::websocket::error::WebsocketError;
use crate::websocket::stream::WebsocketStream;
use crate::websocket::util::base64::Base64Encode;
use crate::websocket::util::sha1::SHA1Hash;
use crate::websocket::MAGIC_STRING;

use crate::http::headers::HeaderName;
use crate::http::{RequestHead, Response, StatusCode};

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
fn handshake(
  request: RequestHead,
  stream: &mut Box<dyn ConnectionStream>,
) -> Result<(), WebsocketError> {
  // Get the handshake key header
  let handshake_key =
    request.get_header("Sec-WebSocket-Key").ok_or(WebsocketError::HandshakeError)?;

  // Calculate the handshake response
  let sec_websocket_accept = format!("{}{}", handshake_key, MAGIC_STRING).hash().encode();

  // Serialise the handshake response
  let response = Response::new(StatusCode::SwitchingProtocols)
    .with_header(HeaderName::Upgrade, "websocket")
    .map_err(|_| WebsocketError::HandshakeError)?
    .with_header(HeaderName::Connection, "Upgrade")
    .map_err(|_| WebsocketError::HandshakeError)?
    .with_header("Sec-WebSocket-Accept", sec_websocket_accept)
    .map_err(|_| WebsocketError::HandshakeError)?;

  // Transmit the handshake response
  response
    .write_to(request.version(), stream.as_stream_write())
    .map_err(|_| WebsocketError::WriteError)?;

  Ok(())
}
