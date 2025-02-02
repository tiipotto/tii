use tii::extras::{builtin_endpoints, Connector, TcpConnector};

use tii::http::request_context::RequestContext;
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;
use tii::websocket::message::WebsocketMessage;
use tii::websocket::stream::{ReadMessageTimeoutResult, WebsocketReceiver, WebsocketSender};
use log::{info, LevelFilter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// App state with a simple global atomic counter
static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn main() -> TiiResult<()> {
  //Install a simple "output" for the log crate, so we can see something in the console.
  //Adjust level if it's too verbose for you.
  colog::default_builder().filter_level(LevelFilter::Trace).init();

  //Visit localhost:8080 in a web-browser like firefox or chrome to see this example.
  let tii_server = TiiBuilder::builder_arc(|builder| {
    builder.router(|router| {
      router
        .route_any("/*", builtin_endpoints::serve_dir("./examples/static/ws"))?
        .ws_route_any("/ws", echo_handler)
    })
  })
  .expect("ERROR");

  let _ = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?.join(None);

  Ok(())
}

/// Handler for WebSocket connections.
/// This is wrapped in `websocket_handler` to manage the handshake for us using the `tii_ws` crate.
fn echo_handler(
  request: &RequestContext,
  mut receiver: WebsocketReceiver,
  sender: WebsocketSender,
) -> TiiResult<()> {
  // Get the address of the client.
  let addr = request.peer_address();

  info!("New connection from {}", addr);

  {
    let sender = sender.clone();
    info!("Starting ping handler thread to ping client every 30s");
    std::thread::spawn(move || loop {
      std::thread::sleep(Duration::from_millis(30_000));
      if sender.is_closed() {
        info!("WebsocketSender is closed, ping handler bailing out...");
        return;
      }
      info!("30 seconds have elapsed, sending ping to client...");
      sender.ping().expect("async ping handler failed");
    });
  }

  // Loop while the client is connected.
  loop {
    // Block up to 5s to receive the next web socket message.
    match receiver.read_message_timeout(Some(Duration::from_millis(5000))) {
      // If the message was received successfully, echo it back with an increasing number at the end.
      Ok(ReadMessageTimeoutResult::Message(message)) => match message {
        WebsocketMessage::Text(text) => {
          let message = text;
          let count = COUNTER.fetch_add(1, Ordering::SeqCst);
          let response = format!("{} {}", message, count);
          sender.text(response)?;
          info!("Received message `{}` from {}, echoing with the number {}", message, addr, count)
        }
        WebsocketMessage::Binary(binary) => {
          info!("Received binary data, echoing data back as is");
          sender.send(WebsocketMessage::Binary(binary))?;
        }
        WebsocketMessage::Ping => {
          info!("Received ping, responding with pong");
          sender.send(WebsocketMessage::Pong)?;
        }
        WebsocketMessage::Pong => {
          info!("Received pong");
        }
      },
      Ok(ReadMessageTimeoutResult::Timeout) => {
        info!("No message received in 5s sending ping...");
        sender.ping()?;
      }
      // If the connection was closed, break out of the loop.
      Ok(ReadMessageTimeoutResult::Closed) => {
        info!("Connection closed by {}", addr);
        return Ok(());
      }
      Err(e) => {
        info!("Websocket Error: {}", e);
        return Ok(());
      }
    }
  }
}
