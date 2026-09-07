//! Provides functionality for working with a WebSocket stream.

use crate::websocket::frame::{Frame, Opcode};
use crate::websocket::message::WebsocketMessage;
use std::collections::VecDeque;
use std::{io, mem};

use crate::stream::ConnectionStream;
use crate::tii_error::{RequestHeadParsingError, TiiError, TiiResult};
use crate::util::{unwrap_poison, unwrap_some};
use crate::{error_log, trace_log, warn_log};
use std::io::{Cursor, ErrorKind, Read, Write};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
enum CloseState {
  Open,
  CloseSent,
  Closed,
}

#[derive(Debug, PartialEq, Eq)]
enum WriteOutcome {
  Written,
  Closing,
}

#[derive(Debug)]
struct WebSocketGuard {
  closed: AtomicBool,
  closing: AtomicBool,
  write_mutex: Mutex<CloseState>,
  stream: Box<dyn ConnectionStream>,
}

impl WebSocketGuard {
  fn is_closed(&self) -> bool {
    self.closed.load(SeqCst) || self.closing.load(SeqCst)
  }

  fn write_frame(&self, opcode: Opcode, payload: &[u8]) -> TiiResult<WriteOutcome> {
    let state = unwrap_poison(self.write_mutex.lock())?;
    let can_write =
      *state == CloseState::Open || (*state == CloseState::CloseSent && opcode == Opcode::Pong);
    if self.closed.load(SeqCst) || !can_write {
      return Ok(WriteOutcome::Closing);
    }
    Frame::write_unowned_payload_frame(self.stream.as_stream_write(), opcode, payload)
      .inspect_err(|e| {
        self.closed.store(true, SeqCst);
        error_log!("WebSocketGuard::write_frame error: {}", e);
      })?;
    Ok(WriteOutcome::Written)
  }

  fn close(&self, reply: Option<&[u8]>) -> TiiResult<()> {
    let mut state = unwrap_poison(self.write_mutex.lock())?;
    if self.closed.load(SeqCst) {
      return Ok(());
    }
    let send = *state == CloseState::Open;
    self.closing.store(true, SeqCst);
    if reply.is_some() {
      *state = CloseState::Closed;
      self.closed.store(true, SeqCst);
    } else if send {
      *state = CloseState::CloseSent;
    } else {
      return Ok(());
    }
    if send {
      Frame::new(Opcode::Close, reply.unwrap_or_default().to_vec())
        .write_to(self.stream.as_stream_write())
        .inspect_err(|_| self.closed.store(true, SeqCst))?;
    }
    Ok(())
  }
}

/// Sending side of a web socket
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct WebsocketSender(Arc<WebSocketGuard>);

/// Creates a new WebSocket receiver sender pair.
pub fn new_web_socket_stream(
  connection: &dyn ConnectionStream,
) -> (WebsocketSender, WebsocketReceiver) {
  let guard = Arc::new(WebSocketGuard {
    closed: AtomicBool::new(false),
    closing: AtomicBool::new(false),
    write_mutex: Mutex::new(CloseState::Open),
    stream: connection.new_ref(),
  });

  let sender = WebsocketSender(guard.clone());

  let receiver = WebsocketReceiver {
    guard,
    state: Vec::new(),
    cursor: Default::default(),
    unhandled_messages: Default::default(),
  };

  (sender, receiver)
}

impl WebsocketSender {
  fn write_frame(&self, opcode: Opcode, payload: &[u8]) -> TiiResult<()> {
    match self.0.write_frame(opcode, payload)? {
      WriteOutcome::Written => Ok(()),
      WriteOutcome::Closing => Err(io::Error::from(ErrorKind::ConnectionReset).into()),
    }
  }

  /// returns true once closing has started or the connection has failed
  #[must_use]
  pub fn is_closed(&self) -> bool {
    self.0.is_closed()
  }

  /// Sends a message to the client.
  pub fn send(&self, message: WebsocketMessage) -> TiiResult<()> {
    match message {
      WebsocketMessage::Text(txt) => self.text(txt),
      WebsocketMessage::Binary(bin) => self.binary(bin),
      WebsocketMessage::Ping => self.ping(),
      WebsocketMessage::Pong => self.pong(),
    }
  }

  /// initiates a close, stopping further application writes.
  pub fn close(&self) -> TiiResult<()> {
    self.0.close(None)
  }

  /// Sends a binary message to the client
  pub fn binary(&self, message: impl Into<Vec<u8>>) -> TiiResult<()> {
    self.write_frame(Opcode::Binary, &message.into())
  }

  /// Sends a text message to the client
  pub fn text(&self, message: impl ToString) -> TiiResult<()> {
    self.write_frame(Opcode::Text, message.to_string().as_bytes())
  }

  /// Sends a ping to the client.
  pub fn ping(&self) -> TiiResult<()> {
    self.write_frame(Opcode::Ping, &[])
  }

  /// Sends an empty pong message to the client.
  pub fn pong(&self) -> TiiResult<()> {
    self.write_frame(Opcode::Pong, &[])
  }

  /// Attempts to get the peer address of this stream.
  pub fn peer_addr(&self) -> TiiResult<String> {
    Ok(self.0.stream.peer_addr()?)
  }
}

/// Receiving side of a web socket
#[derive(Debug)]
pub struct WebsocketReceiver {
  guard: Arc<WebSocketGuard>,
  state: Vec<Frame>,
  cursor: Cursor<Vec<u8>>,
  unhandled_messages: VecDeque<WebsocketMessage>,
}

/// Return enum for the fn WebsocketReceiver::read_message_timeout
#[derive(Debug)]
pub enum ReadMessageTimeoutResult {
  /// We got a message without running into any timeout
  Message(WebsocketMessage),
  /// We got a timeout before the first byte of the next message was received.
  Timeout,
  /// We received the Close 'Message' without running into any timeout
  Closed,
}

impl WebsocketReceiver {
  #[cfg(feature = "extras")]
  pub(crate) fn read_timeout(&self) -> TiiResult<Option<Duration>> {
    Ok(self.guard.stream.get_read_timeout()?)
  }

  /// initializes close, stopping further application writes.
  pub fn close(&self) -> TiiResult<()> {
    self.guard.close(None)
  }

  /// If the WebsocketReceiver is used with the "io::Read" trait then
  /// any ping/pong messages received are not handled. They are instead queued.
  /// This fn pop_front's the head of the queue.
  pub fn unhandled(&mut self) -> Option<WebsocketMessage> {
    self.unhandled_messages.pop_front()
  }

  /// receive the next complete message.
  /// Ok(None) indicates that the web socket is closed.
  pub fn read_message(&mut self) -> TiiResult<Option<WebsocketMessage>> {
    if let Some(message) = self.unhandled_messages.pop_front() {
      return Ok(Some(message));
    }

    self.read_next_frame()
  }

  /// This fn waits until timeout expires before the first byte of the next Message is received.
  ///
  /// The specified timeout is completely independent of the read timeout of the TiiServer.
  /// Values where timeout.is_zero() returns true may cause Err to be returned depending on how the
  /// underlying connection treats this value.
  ///
  /// The actual reading of the Message is still subject to the normal timeout mechanics.
  /// Should the client pause in the middle of a frame before sending the rest of it then
  /// this fn will return the fatal error Err(TimedOut).
  ///
  /// Passing None for timeout means Infinite timeout until either the client closes the connection
  /// sends a byte or the OS reset the connection;
  ///
  pub fn read_message_timeout(
    &mut self,
    timeout: Option<Duration>,
  ) -> TiiResult<ReadMessageTimeoutResult> {
    if let Some(message) = self.unhandled_messages.pop_front() {
      return Ok(ReadMessageTimeoutResult::Message(message));
    }

    if self.guard.stream.available() == 0 {
      if self.guard.closed.load(SeqCst) {
        return Ok(ReadMessageTimeoutResult::Closed);
      }

      let old_timeout = self.guard.stream.get_read_timeout()?.as_ref().cloned();
      if let Err(err) = self.guard.stream.set_read_timeout(timeout) {
        let err = TiiError::from(err);
        self.guard.closed.store(true, SeqCst);
        error_log!("WebsocketReceiver::read_message_timeout error setting timeout for 1st byte of next frame {}", &err);
        return Err(err);
      }
      let res = self.guard.stream.ensure_readable();
      let res2 = self.guard.stream.set_read_timeout(old_timeout);

      if let Err(err) = res2 {
        let err = TiiError::from(err);
        self.guard.closed.store(true, SeqCst);
        error_log!("WebsocketReceiver::read_message_timeout error setting timeout back to read timeout after waiting for 1st byte of next frame {}", &err);
        return Err(err);
      }

      if let Err(err) = res {
        if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) {
          return Ok(ReadMessageTimeoutResult::Timeout);
        }
        self.guard.closed.store(true, SeqCst);
        error_log!("WebsocketReceiver::read_message_timeout error while waiting for 1st byte of next frame {}", &err);
        return Err(TiiError::from(err));
      }
    }

    match self.read_next_frame() {
      Ok(Some(message)) => Ok(ReadMessageTimeoutResult::Message(message)),
      Ok(None) => Ok(ReadMessageTimeoutResult::Closed),
      Err(err) => Err(err),
    }
  }

  /// Attempts to read a message from the given stream.
  ///
  /// Silently responds to pings with pongs, as specified in [RFC 6455 Section 5.5.2](https://datatracker.ietf.org/doc/html/rfc6455#section-5.5.2).
  fn read_next_frame(&mut self) -> TiiResult<Option<WebsocketMessage>> {
    if self.guard.closed.load(SeqCst) {
      self.state.clear();
      return Ok(None);
    }

    let as_read = self.guard.stream.as_stream_read();
    // Keep reading frames until we get the finish frame
    while self.state.last().map(|f| !f.fin).unwrap_or(true) {
      if self.guard.closed.load(SeqCst) {
        self.state.clear();
        return Ok(None);
      }
      let frame = Frame::from_stream(as_read).inspect_err(|e| {
        self.guard.closed.store(true, SeqCst);
        error_log!("WebsocketReceiver::read_next_frame Frame::from_stream error: {}", e);
      })?;

      if self.guard.closed.load(SeqCst) {
        self.state.clear();
        return Ok(None);
      }

      if frame.opcode == Opcode::Close {
        self.state.clear();
        frame.validate_close_payload().inspect_err(|_| {
          self.guard.closed.store(true, SeqCst);
        })?;
        self.guard.close(Some(&frame.payload))?;
        return Ok(None);
      }

      if frame.opcode == Opcode::Ping {
        if self.guard.write_frame(Opcode::Pong, &frame.payload)? == WriteOutcome::Written {
          return Ok(Some(WebsocketMessage::Ping));
        }
        continue;
      }

      if self.guard.closing.load(SeqCst) {
        self.state.clear();
        continue;
      }

      if frame.opcode == Opcode::Pong {
        return Ok(Some(WebsocketMessage::Pong));
      }

      self.state.push(frame);
    }

    let frames = mem::take(&mut self.state);
    let frame_type = unwrap_some(frames.first()).opcode;

    let size = frames.iter().map(|f| f.payload.len()).sum();
    let mut payload = Vec::with_capacity(size);

    for (idx, frame) in frames.into_iter().enumerate() {
      if idx != 0 && frame.opcode != Opcode::Continuation {
        return Err(TiiError::RequestHeadParsing(
          RequestHeadParsingError::UnexpectedWebSocketOpcode,
        ));
      }
      payload.extend_from_slice(frame.payload.as_slice());
    }

    match frame_type {
      Opcode::Text => {
        let payload = String::from_utf8(payload).map_err(|e| {
          self.guard.closed.store(true, SeqCst);
          TiiError::RequestHeadParsing(RequestHeadParsingError::WebSocketTextMessageIsNotUtf8(
            e.into_bytes(),
          ))
        })?;

        Ok(Some(WebsocketMessage::Text(payload)))
      }
      Opcode::Binary => Ok(Some(WebsocketMessage::Binary(payload))),
      _ => {
        self.guard.closed.store(true, SeqCst);
        Err(TiiError::RequestHeadParsing(RequestHeadParsingError::UnexpectedWebSocketOpcode))
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
              unwrap_some(buf.get_mut(..bytes.len())).copy_from_slice(bytes);
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
    self.write_frame(Opcode::Binary, buf)?;
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

impl Drop for WebSocketGuard {
  fn drop(&mut self) {
    trace_log!("WebsocketReceiver::drop");
    if self.closed.load(SeqCst) {
      trace_log!("WebsocketReceiver::drop already closed");
      return;
    }

    trace_log!("WebsocketReceiver::drop closing...");
    if let Err(err) = self.close(None) {
      warn_log!("WebsocketSender::drop error: {}", err);
    }
    trace_log!("WebsocketReceiver::drop closed.");
  }
}
