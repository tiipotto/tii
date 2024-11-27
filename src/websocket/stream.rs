//! Provides functionality for working with a WebSocket stream.

use crate::websocket::frame::{Frame, Opcode};
use crate::websocket::message::WebsocketMessage;
use std::collections::VecDeque;
use std::{io, mem};

use crate::humpty_error::{HumptyError, HumptyResult, RequestHeadParsingError};
use crate::stream::ConnectionStream;
use crate::util::unwrap_some;
use std::io::{Cursor, Read, Write};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

/// Sending side of a web socket
pub struct WebsocketSender {
  closed: Arc<AtomicBool>,
  stream: Box<dyn ConnectionStream>,
}

/// Creates a new WebSocket receiver sender pair.
pub fn new(connection: &dyn ConnectionStream) -> (WebsocketSender, WebsocketReceiver) {
  let closed = Arc::new(AtomicBool::new(false));
  let dup = connection.new_ref();
  let sender = WebsocketSender { closed: Arc::clone(&closed), stream: dup };

  let receiver = WebsocketReceiver {
    closed,
    state: Vec::new(),
    stream: connection.new_ref(),
    cursor: Default::default(),
    unhandled_messages: Default::default(),
  };

  (sender, receiver)
}

impl WebsocketSender {
  /// Sends a message to the client.
  pub fn send(&self, message: WebsocketMessage) -> HumptyResult<()> {
    match message {
      WebsocketMessage::Text(txt) => self.text(txt),
      WebsocketMessage::Binary(bin) => self.binary(bin),
      WebsocketMessage::Ping => self.ping(),
      WebsocketMessage::Pong => self.pong(),
    }
  }

  /// Sends a binary message to the client
  pub fn binary(&self, message: impl Into<Vec<u8>>) -> HumptyResult<()> {
    Frame::new(Opcode::Binary, message.into()).write_to(self.stream.as_stream_write())
  }

  /// Sends a text message to the client
  pub fn text(&self, message: impl ToString) -> HumptyResult<()> {
    Frame::new(Opcode::Text, message.to_string().into_bytes())
      .write_to(self.stream.as_stream_write())
  }

  /// Sends a ping to the client.
  pub fn ping(&self) -> HumptyResult<()> {
    Frame::new(Opcode::Ping, Vec::new()).write_to(self.stream.as_stream_write())
  }

  /// Sends a pong message to the client.
  pub fn pong(&self) -> HumptyResult<()> {
    Frame::new(Opcode::Ping, Vec::new()).write_to(self.stream.as_stream_write())
  }

  /// Attempts to get the peer address of this stream.
  pub fn peer_addr(&self) -> HumptyResult<String> {
    Ok(self.stream.peer_addr()?)
  }
}

/// Receiving side of a web socket
pub struct WebsocketReceiver {
  closed: Arc<AtomicBool>,
  state: Vec<Frame>,
  stream: Box<dyn ConnectionStream>,
  cursor: Cursor<Vec<u8>>,
  unhandled_messages: VecDeque<WebsocketMessage>,
}

impl WebsocketReceiver {
  /// If the WebsocketReceiver is used with the "io::Read" trait then
  /// any ping/pong messages received are not handled. They are instead queued.
  /// This fn pop_front's the head of the queue.
  pub fn unhandled(&mut self) -> Option<WebsocketMessage> {
    self.unhandled_messages.pop_front()
  }

  /// receive the next complete message.
  pub fn recv(&mut self) -> HumptyResult<Option<WebsocketMessage>> {
    if let Some(message) = self.unhandled_messages.pop_front() {
      return Ok(Some(message));
    }

    self.read_next_frame()
  }

  /// Attempts to read a message from the given stream.
  ///
  /// Silently responds to pings with pongs, as specified in [RFC 6455 Section 5.5.2](https://datatracker.ietf.org/doc/html/rfc6455#section-5.5.2).
  fn read_next_frame(&mut self) -> HumptyResult<Option<WebsocketMessage>> {
    if self.closed.load(SeqCst) {
      return Ok(None);
    }

    let as_read = self.stream.as_stream_read();
    // Keep reading frames until we get the finish frame
    while self.state.last().map(|f| !f.fin).unwrap_or(true) {
      let frame = Frame::from_stream(as_read)?;

      if frame.opcode == Opcode::Ping {
        return Ok(Some(WebsocketMessage::Ping));
      }

      if frame.opcode == Opcode::Pong {
        return Ok(Some(WebsocketMessage::Pong));
      }

      if frame.opcode == Opcode::Close {
        self.closed.store(true, SeqCst);
        if self.state.is_empty() {
          return Ok(None);
        }

        return Err(HumptyError::RequestHeadParsing(
          RequestHeadParsingError::WebSocketClosedDuringPendingMessage,
        ));
      }

      self.state.push(frame);
    }

    let frames = mem::take(&mut self.state);
    let frame_type = unwrap_some(frames.first()).opcode;

    let size = frames.iter().map(|f| f.payload.len()).sum();
    let mut payload = Vec::with_capacity(size);

    for (idx, frame) in frames.into_iter().enumerate() {
      if idx != 0 && frame.opcode != Opcode::Continuation {
        return Err(HumptyError::RequestHeadParsing(
          RequestHeadParsingError::UnexpectedWebSocketOpcode,
        ));
      }
      payload.extend_from_slice(frame.payload.as_slice());
    }

    match frame_type {
      Opcode::Text => {
        let payload = String::from_utf8(payload).map_err(|e| {
          self.closed.store(true, SeqCst);
          HumptyError::RequestHeadParsing(RequestHeadParsingError::WebSocketTextMessageIsNotUtf8(
            e.into_bytes(),
          ))
        })?;

        Ok(Some(WebsocketMessage::Text(payload)))
      }
      Opcode::Binary => Ok(Some(WebsocketMessage::Binary(payload))),
      _ => {
        self.closed.store(true, SeqCst);
        Err(HumptyError::RequestHeadParsing(RequestHeadParsingError::UnexpectedWebSocketOpcode))
      }
    }
  }
}

impl Read for WebsocketReceiver {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    loop {
      let cnt = self.cursor.read(buf)?;
      if cnt != 0 {
        return Ok(cnt);
      }

      return match self.read_next_frame() {
        Ok(Some(message)) => match message.bytes() {
          Some(bytes) => {
            if bytes.len() <= buf.len() {
              buf[..bytes.len()].copy_from_slice(bytes);
              Ok(bytes.len())
            } else {
              self.cursor = Cursor::new(bytes.to_vec());
              continue;
            }
          }
          None => {
            self.unhandled_messages.push_back(message);
            continue;
          }
        },
        Ok(None) => Ok(0),
        Err(err) => {
          return Err(err.into());
        }
      };
    }
  }
}

impl Write for WebsocketSender {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    //TODO this does not need to be copied!
    self.binary(buf.to_vec())?;
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

impl Drop for WebsocketSender {
  fn drop(&mut self) {
    if !self.closed.load(SeqCst) {
      self.stream.write_all(Frame::new(Opcode::Close, Vec::new()).as_ref()).ok();
    }
  }
}
