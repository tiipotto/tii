use humpty::extras::builtin_endpoints;
use humpty::extras::tcp_app;

use humpty::http::request_context::RequestContext;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use humpty::websocket::message::WebsocketMessage;
use humpty::websocket::stream::{WebsocketReceiver, WebsocketSender};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

/// App state with a simple global atomic counter
static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn main() -> Result<(), Box<dyn Error>> {
  let humpty_server = HumptyBuilder::builder_arc(|builder| {
    builder.router(|router| {
      router
        .route_any("/*", builtin_endpoints::serve_dir("./examples/static/ws"))?
        .ws_route_any("/ws", echo_handler)
    })
  })
  .expect("ERROR");

  let _ = tcp_app::App::new("0.0.0.0:8080", humpty_server)?.run();

  Ok(())
}

/// Handler for WebSocket connections.
/// This is wrapped in `websocket_handler` to manage the handshake for us using the `humpty_ws` crate.
fn echo_handler(
  request: &RequestContext,
  mut receiver: WebsocketReceiver,
  sender: WebsocketSender,
) -> HumptyResult<()> {
  // Get the address of the client.
  let addr = request.peer_address();

  println!("New connection from {}", addr);

  // Loop while the client is connected.
  loop {
    // Block while waiting to receive a message.
    match receiver.recv() {
      // If the message was received successfully, echo it back with an increasing number at the end.
      Ok(Some(message)) => match message {
        WebsocketMessage::Text(text) => {
          let message = text;
          let count = COUNTER.fetch_add(1, Ordering::SeqCst);
          let response = format!("{} {}", message, count);
          sender.text(response).unwrap();
          println!(
            "Received message `{}` from {}, echoing with the number {}",
            message, addr, count
          )
        }
        WebsocketMessage::Binary(binary) => {
          println!("Received binary data, echoing data back as is");
          sender.send(WebsocketMessage::Binary(binary)).unwrap();
        }
        WebsocketMessage::Ping => {
          println!("Received ping, responding with pong");
          sender.send(WebsocketMessage::Pong).unwrap();
        }
        WebsocketMessage::Pong => {
          println!("Received pong");
        }
      },
      // If the connection was closed, break out of the loop and clean up
      Ok(None) => {
        break;
      }
      // Ignore any other errors
      _ => (),
    }
  }

  println!("Connection closed by {}", addr);
  Ok(())
}
