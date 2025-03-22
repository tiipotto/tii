use std::error::Error;
use std::thread::spawn;
use std::time::Duration;

use tii::extras::{Connector, TcpConnector, WSBAppBuilder, WsbHandle};
use tii::{ServerBuilder, WebsocketMessage};

fn main() -> Result<(), Box<dyn Error>> {
  trivial_log::init_std(log::LevelFilter::Debug).unwrap();

  let websocket_linker = WSBAppBuilder::default().with_message_handler(message_handler);

  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder
      .router(|router| router.ws_route_any("/ws", websocket_linker.endpoint()))?
      .with_connection_timeout(Some(Duration::from_secs(8)))
  })
  .unwrap();

  let _websocket_thread = spawn(|| {
    websocket_linker.finalize().run().unwrap();
  });

  let app = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server).unwrap();

  // we never exit, autobahn takes a long time
  app.join(None);
  trivial_log::free();
  Ok(())
}

fn message_handler(handle: WsbHandle, message: WebsocketMessage) {
  handle.send(message)
}
