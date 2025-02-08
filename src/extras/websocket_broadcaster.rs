use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::{io, thread, time::Duration};

use crate::{error_log, info_log, util, warn_log};
use crate::{TiiReadMessageTimeoutResult, TiiWebsocketReceiver, TiiWebsocketSender};
use crate::{TiiRequestContext, TiiWebsocketEndpoint, TiiWebsocketMessage};

type WebsocketContext = (TiiWebsocketReceiver, TiiWebsocketSender, String);

#[derive(Debug)]
#[non_exhaustive]
/// Top level Error for why `run` ran into a fatal crash.
/// Use the `log` feature for debugging/tracing errors (eg. a WS client disconnecting).
pub enum TiiWsbAppError {
  /// Broadcast thread unexpected exit
  BroadcastThread(Result<(), io::Error>),
  /// Exec thread unexpected exit
  ExecThread(Result<(), io::Error>),
  /// Panic, use `log` feature to debug
  Panic,
}

/// WebSocketApp builder/linker for setup and linking to Tii
pub struct TiiWsbAppBuilder {
  tii_link: Arc<Mutex<Sender<WebsocketContext>>>,
  state: State,
}

/// Represents a WebSocket app.
pub struct TiiWsbApp {
  state: State,
}

// internal app structure
struct State {
  // A receiver to receive new WebsocketStreams from TiiServer.
  incoming_streams: Receiver<WebsocketContext>,

  heartbeat: Option<Duration>,

  // A Vec of all the streams for broadcasting.
  send_streams: Arc<Mutex<Vec<Sender<TiiWsbOutgoingMessage>>>>,
  // A sender which is used by handler threads to send messages to clients.
  broadcast_sender: Sender<TiiWebsocketMessage>,
  // A receiver which receives messages from handler threads to forward to clients.
  outgoing_broadcasts: Receiver<TiiWebsocketMessage>,

  // The event handler called when a new client connects.
  connect_handler: Option<Box<dyn TiiWsbEventHandler>>,
  // The event handler called when a client disconnects.
  disconnect_handler: Option<Box<dyn TiiWsbEventHandler>>,
  // The event handler called when a client sends a message.
  message_handler: Option<Box<dyn TiiWsbMessageHandler>>,

  // Shutdown signal for the application.
  shutdown: Option<Receiver<()>>,
  shutdown_flag: Arc<AtomicBool>,
}

/// Represents a WebSocketApp handle.
///
/// This is what is passed to the handler in place of the actual stream. It is able to send
/// messages back to the stream or broadcast to all streams within the WebSocketApp.
#[derive(Debug)]
pub struct TiiWsbHandle {
  addr: String,
  sender: Sender<TiiWsbOutgoingMessage>,
}

/// Represents a global sender which can be used to broadcast messages to all clients.
pub struct BroadcastSender(Sender<TiiWebsocketMessage>);

impl BroadcastSender {
  /// Broadcast a message to all connected clients.
  pub fn broadcast(&self, message: TiiWebsocketMessage) {
    self.0.send(message).ok();
  }
}

/// Represents a message to be sent from the server (tii) to client(s).
#[derive(Debug)]
pub enum TiiWsbOutgoingMessage {
  /// A message to be sent to a specific client.
  Message(TiiWebsocketMessage),
  /// A message to be sent to every connected client.
  Broadcast(TiiWebsocketMessage),
}

/// Represents a function able to handle a WebSocket event (a connection or disconnection).
/// It is passed the stream which triggered the event.
///
/// ## Example
/// A basic example of an event handler would be as follows:
/// ```
/// fn connection_handler(stream: &tii::extras::TiiWsbHandle) {
///     println!("A new client connected! {:?}", stream.peer_addr());
///
///     stream.send(tii::TiiWebsocketMessage::new_text("Hello, World!"));
/// }
/// ```
pub trait TiiWsbEventHandler: Fn(TiiWsbHandle) + Send + Sync + 'static {}
impl<T> TiiWsbEventHandler for T where T: Fn(TiiWsbHandle) + Send + Sync + 'static {}

/// Represents a function able to handle a message event.
/// It is passed the stream which sent the message.
///
/// ## Example
/// A basic example of a message handler would be as follows:
/// ```
/// use tii::TiiWebsocketMessage;
/// use tii::extras::TiiWsbHandle;
/// fn message_handler(handle: TiiWsbHandle, message: TiiWebsocketMessage) {
///    println!("{:?}", message);
///
///    handle.send(TiiWebsocketMessage::new_text("Message received."));
/// }
/// ```
pub trait TiiWsbMessageHandler:
  Fn(TiiWsbHandle, TiiWebsocketMessage) + Send + Sync + 'static
{
}
impl<T> TiiWsbMessageHandler for T where
  T: Fn(TiiWsbHandle, TiiWebsocketMessage) + Send + Sync + 'static
{
}

impl Default for TiiWsbAppBuilder {
  fn default() -> Self {
    let (connect_hook, incoming_streams) = channel();
    let (broadcast_sender, outgoing_broadcasts) = channel();

    Self {
      tii_link: Arc::new(Mutex::new(connect_hook)),
      state: State {
        heartbeat: Some(Duration::from_secs(5)),
        send_streams: Default::default(),
        outgoing_broadcasts,
        broadcast_sender,
        incoming_streams,
        connect_handler: None,
        disconnect_handler: None,
        message_handler: None,
        shutdown: None,
        shutdown_flag: Arc::new(AtomicBool::new(false)),
      },
    }
  }
}

impl TiiWsbAppBuilder {
  /// Returns the finalized App.
  pub fn finalize(self) -> TiiWsbApp {
    TiiWsbApp { state: self.state }
  }

  /// Returns a websocket endpoint that will service this WebSocketBroadcastApp.
  /// Add this endpoint to a websocket route in a TiiBuilder.
  pub fn endpoint(&self) -> impl TiiWebsocketEndpoint {
    let hook = self.tii_link.clone();
    move |request: &TiiRequestContext,
          receiver: TiiWebsocketReceiver,
          sender: TiiWebsocketSender| {
      let hook = util::unwrap_poison(hook.lock());
      Ok(hook?.send((receiver, sender, request.peer_address().to_string()))?)
    }
  }

  /// Returns a new `BroadcastSender`, which can be used to send messages.
  pub fn sender(&self) -> BroadcastSender {
    BroadcastSender(self.state.broadcast_sender.clone())
  }

  /// Set the event handler called when a new client connects.
  pub fn with_connect_handler(mut self, handler: impl TiiWsbEventHandler) -> Self {
    self.state.connect_handler = Some(Box::new(handler));
    self
  }

  /// Set the event handler called when a client disconnects.
  pub fn with_disconnect_handler(mut self, handler: impl TiiWsbEventHandler) -> Self {
    self.state.disconnect_handler = Some(Box::new(handler));
    self
  }

  /// Set the message handler called when a client sends a message.
  pub fn with_message_handler(mut self, handler: impl TiiWsbMessageHandler) -> Self {
    self.state.message_handler = Some(Box::new(handler));
    self
  }

  /// Sets the heartbeat configuration for the app.
  ///
  /// By default, this is 5 seconds.
  /// It is highly recommended to set this reasonably shorter than your `with_connection_timeout`.
  pub fn with_heartbeat(mut self, heartbeat: Duration) -> Self {
    self.state.heartbeat = Some(heartbeat);
    self
  }

  /// Registers a shutdown signal to gracefully shutdown the app
  ///
  /// For a full/consistent shutdown, you must set both
  ///`TiiBuilder::with_connection_timeout` and `with_heartbeat`
  ///
  /// Threads are fully joined, but won't exit until timeouts/heartbeats.
  pub fn with_shutdown(mut self, shutdown_receiver: Receiver<()>) -> Self {
    self.state.shutdown = Some(shutdown_receiver);
    self
  }
}

impl TiiWsbApp {
  /// Start the application on the main thread.
  /// This blocks until the Tii server has been dropped.
  pub fn run(self) -> Result<(), TiiWsbAppError> {
    let connect_handler = self.state.connect_handler.map(Arc::new);
    let disconnect_handler = self.state.disconnect_handler.map(Arc::new);
    let message_handler = self.state.message_handler.map(Arc::new);
    let streams = self.state.send_streams.clone();

    let timeout = {
      if let Some(hb) = self.state.heartbeat {
        hb
      } else {
        Duration::MAX
      }
    };

    // broadcast/heartbeat thread
    let sd_flag = self.state.shutdown_flag.clone();
    let broadcast_thread = thread::spawn(move || {
      loop {
        if sd_flag.load(Ordering::SeqCst) {
          break;
        }
        let recv = self.state.outgoing_broadcasts.recv_timeout(timeout);

        // Remove up to one idx per broadcast. They should eventually all be cleaned up because of the heartbeat.
        let mut remove_idx = None;
        match recv {
          Ok(message) => {
            let mut streams = util::unwrap_poison(streams.lock())?;
            for (idx, stream) in streams.iter_mut().enumerate() {
              // convert the broadcast back to message, but for each sender
              if stream.send(TiiWsbOutgoingMessage::Message(message.clone())).is_err() {
                remove_idx = Some(idx);
              }
            }
          }
          // the WebSocketApp has closed
          Err(mpsc::RecvTimeoutError::Disconnected) => break,
          Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some(idx) = remove_idx {
          let mut streams = util::unwrap_poison(streams.lock())?;
          if streams.len() > idx {
            streams.remove(idx);
          }
        }
      }
      Ok::<(), io::Error>(())
    });

    let sd_flag = self.state.shutdown_flag.clone();
    let exec_thread = thread::spawn(move || {
      let mut threads = Vec::new();
      loop {
        if let Some(sd) = &self.state.shutdown {
          if sd.try_recv().is_ok() {
            info_log!("shutdown received in WebSocketApp");
            break;
          }
        }

        let recv = self.state.incoming_streams.recv_timeout(timeout);
        let new_stream = match recv {
          Ok(ns) => ns,
          Err(RecvTimeoutError::Timeout) => continue,
          // The TiiServer has exit, so we tell everybody to exit
          Err(RecvTimeoutError::Disconnected) => {
            info_log!("WebsocketApp initializing shutdown, due to Tii exiting");
            sd_flag.store(true, Ordering::SeqCst);
            break;
          }
        };

        let sender = self.state.broadcast_sender.clone();
        let (message_sender, outgoing_messages) = channel();
        util::unwrap_poison(self.state.send_streams.lock())?.push(message_sender.clone());

        let connect_handler = connect_handler.clone();
        let disconnect_handler = disconnect_handler.clone();
        let message_handler = message_handler.clone();

        let sd_flag = sd_flag.clone();
        threads.push(thread::spawn(move || {
          exec(ExecState {
            stream: new_stream,
            broadcast: sender,
            message_sender,
            outgoing_messages,
            connect_handler,
            disconnect_handler,
            message_handler,
            timeout,
            shutdown_signal: sd_flag,
          });
        }));

        // Iterate over the threads and remove the finished ones
        threads.retain(|handle| !handle.is_finished());
      }

      for t in threads {
        let j = t.join();
        if let Err(e) = j {
          warn_log!("{:?} while doing join of `exec` thread.", e);
        }
      }
      Ok::<(), io::Error>(())
    });

    // monitor for unexpected thread exits, log, and report the AppError
    loop {
      if self.state.shutdown_flag.load(Ordering::SeqCst) {
        break;
      }
      if exec_thread.is_finished() {
        return match exec_thread.join() {
          Ok(et) => Err(TiiWsbAppError::ExecThread(et)),
          Err(e) => {
            error_log!("Unexpected exec_thread panic: {:?}.", e);
            Err(TiiWsbAppError::Panic)
          }
        };
      }
      if broadcast_thread.is_finished() {
        return match exec_thread.join() {
          Ok(bt) => Err(TiiWsbAppError::BroadcastThread(bt)),
          Err(e) => {
            error_log!("Unexpected broadcast_thread panic: {:?}.", e);
            Err(TiiWsbAppError::Panic)
          }
        };
      }

      thread::sleep(timeout);
    }

    if let Err(e) = exec_thread.join() {
      error_log!("{:?} while doing join of `exec` thread.", e);
      return Err(TiiWsbAppError::Panic);
    }

    if let Err(e) = broadcast_thread.join() {
      error_log!("{:?} while doing join of `exec` thread.", e);
      return Err(TiiWsbAppError::Panic);
    }
    Ok(())
  }
}

impl TiiWsbHandle {
  /// Create a new handle.
  pub fn new(addr: String, sender: Sender<TiiWsbOutgoingMessage>) -> Self {
    Self { addr, sender }
  }

  /// Send a message to the client.
  pub fn send(&self, message: TiiWebsocketMessage) {
    self.sender.send(TiiWsbOutgoingMessage::Message(message)).ok();
  }

  /// Broadcast a message to all connected clients.
  pub fn broadcast(&self, message: TiiWebsocketMessage) {
    self.sender.send(TiiWsbOutgoingMessage::Broadcast(message)).ok();
  }

  /// Get the address of the stream.
  pub fn peer_addr(&self) -> String {
    self.addr.clone()
  }
}

struct ExecState {
  stream: WebsocketContext,
  broadcast: Sender<TiiWebsocketMessage>,
  message_sender: Sender<TiiWsbOutgoingMessage>,
  outgoing_messages: Receiver<TiiWsbOutgoingMessage>,
  connect_handler: Option<Arc<Box<dyn TiiWsbEventHandler>>>,
  disconnect_handler: Option<Arc<Box<dyn TiiWsbEventHandler>>>,
  message_handler: Option<Arc<Box<dyn TiiWsbMessageHandler>>>,
  timeout: Duration,
  shutdown_signal: Arc<AtomicBool>,
}

fn exec(es: ExecState) {
  let (mut ws_receiver, ws_sender, addr) = (es.stream.0, es.stream.1, es.stream.2);

  if let Some(ch) = es.connect_handler {
    let handle = TiiWsbHandle::new(addr.clone(), es.message_sender.clone());
    (ch)(handle);
  }

  // write thread
  let write_shutdown = es.shutdown_signal.clone();
  let write_thread = thread::spawn(move || loop {
    if write_shutdown.load(Ordering::SeqCst) {
      break;
    }
    match es.outgoing_messages.recv_timeout(es.timeout) {
      Ok(m) => match m {
        TiiWsbOutgoingMessage::Message(message) => {
          if ws_sender.send(message).is_err() {
            break;
          }
        }
        TiiWsbOutgoingMessage::Broadcast(message) => {
          if es.broadcast.send(message).is_err() {
            break;
          }
        }
      },
      Err(RecvTimeoutError::Disconnected) => break,
      Err(RecvTimeoutError::Timeout) => {
        if ws_sender.ping().is_err() {
          break;
        }
      }
    }
  });

  // read thread
  let read_thread = thread::spawn(move || loop {
    if es.shutdown_signal.load(Ordering::SeqCst) {
      break;
    }
    let Some(ref mh) = es.message_handler else { break };
    match ws_receiver.read_message_timeout(Some(es.timeout)) {
      Ok(message) => match message {
        TiiReadMessageTimeoutResult::Message(m) => {
          match m {
            TiiWebsocketMessage::Binary(_) | TiiWebsocketMessage::Text(_) => {
              (mh)(TiiWsbHandle::new(addr.clone(), es.message_sender.clone()), m);
            }
            TiiWebsocketMessage::Ping => {
              if es
                .message_sender
                .send(TiiWsbOutgoingMessage::Message(TiiWebsocketMessage::Pong))
                .is_err()
              {
                break;
              }
            }
            TiiWebsocketMessage::Pong => (), // do nothing
          }
        }
        TiiReadMessageTimeoutResult::Timeout | TiiReadMessageTimeoutResult::Closed => {
          if let Some(dh) = es.disconnect_handler {
            (dh)(TiiWsbHandle::new(addr.clone(), es.message_sender.clone()));
          }
          break;
        }
      },
      Err(e) => {
        error_log!("ws_app read: {:?} occurred", &e);
        if let Some(dh) = es.disconnect_handler {
          (dh)(TiiWsbHandle::new(addr.clone(), es.message_sender.clone()));
        }
        break;
      }
    }
  });

  if let Err(e) = read_thread.join() {
    error_log!("ws_app read: {:?} occurred", &e);
  }
  if let Err(e) = write_thread.join() {
    error_log!("ws_app read: {:?} occurred", &e);
  }
}
