use crate::http::RequestHead;
use crate::stream::ConnectionStream;
use crate::websocket::handler::handshake;
use crate::websocket::WebsocketStream;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// Provides asynchronous WebSocket functionality.
/// Supply a hook to an asynchronous WebSocket app to handle the subsequent messages.
///
/// It is important to note that, unless you need to modify the underlying Humpty application, it is
///   easier to simply create a regular app with `AsyncWebsocketApp::new()` which manages the Humpty
///   application internally.
///
pub fn async_websocket_handler(
  hook: Arc<Mutex<Sender<WebsocketStream>>>,
) -> impl Fn(RequestHead, Box<dyn ConnectionStream>) {
  move |request: RequestHead, mut stream: Box<dyn ConnectionStream>| {
    if handshake(request, &mut stream).is_ok() {
      hook.lock().unwrap().send(WebsocketStream::new(stream)).ok();
    }
  }
}
