//! Provides an abstraction over WebSocket frames called `Message`.

/// Represents a WebSocket message.
#[derive(Debug, Clone)]
pub enum WebsocketMessage {
  /// UTF-8 Text message
  Text(String),
  /// Binary data message
  Binary(Vec<u8>),
  /// Ping message
  Ping,
  /// Pong message
  Pong,
}

impl WebsocketMessage {
  /// Creates a new binary message with the given payload.
  pub fn new_binary<T>(payload: T) -> Self
  where
    T: Into<Vec<u8>>,
  {
    Self::Binary(payload.into())
  }

  /// Creates a new Web-Socket text message
  pub fn new_text(str: impl ToString) -> Self {
    Self::Text(str.to_string())
  }

  /// Returns whether the sender of this message specified that it contains text.
  pub fn is_text(&self) -> bool {
    matches!(self, Self::Text(_))
  }

  /// Returns the payload as a string, if possible.
  pub fn text(&self) -> Option<&str> {
    match self {
      WebsocketMessage::Text(txt) => Some(txt),
      WebsocketMessage::Binary(bin) => std::str::from_utf8(bin.as_slice()).ok(),
      _ => None,
    }
  }

  /// Returns the payload as a slice of bytes.
  pub fn bytes(&self) -> Option<&[u8]> {
    match self {
      WebsocketMessage::Text(txt) => Some(txt.as_bytes()),
      WebsocketMessage::Binary(bin) => Some(bin.as_slice()),
      WebsocketMessage::Ping => None,
      WebsocketMessage::Pong => None,
    }
  }
}
