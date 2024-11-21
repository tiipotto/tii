use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::response_body::ResponseBody;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use humpty::HumptyTlsStream;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, ServerConnection};
use rustls_pemfile::{certs, private_key};
use std::error::Error;
use std::io::{BufReader, Cursor};
use std::net::TcpListener;
use std::sync::Arc;
use std::{io, thread};

fn load_certs() -> Vec<CertificateDer<'static>> {
  let keyfile = include_bytes!("./ssl/cert.pem").to_vec(); //Use a real cert!
  let mut reader = BufReader::new(Cursor::new(keyfile));
  let n: Vec<_> = certs(&mut reader).map(|e| e.unwrap()).collect();
  n
}

fn load_private_key() -> PrivateKeyDer<'static> {
  let keyfile = include_bytes!("./ssl/key.pem").to_vec(); //Use a real key!

  let mut reader = BufReader::new(Cursor::new(keyfile));
  private_key(&mut reader)
    .expect("Cannot read private key file")
    .expect("Cannot read private key file")
}

fn create_server_config() -> Arc<ServerConfig> {
  let certs = load_certs();
  let key = load_private_key();

  let config =
    ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key).expect("Error");
  Arc::new(config)
}

fn main() -> Result<(), Box<dyn Error>> {
  let app = HumptyBuilder::default().router(|r| r.route_any("/ssl", ssl_route)).build_arc();
  let config = create_server_config();

  let listen = TcpListener::bind("0.0.0.0:8080")?;
  for stream in listen.incoming() {
    let app = app.clone();
    let config = config.clone();
    thread::spawn(move || {
      let tls = HumptyTlsStream::create_unpooled(
        stream?,
        ServerConnection::new(config).expect("TLS ERROR"),
      )?;
      app.handle_connection(tls).expect("ERORR");
      Ok::<(), io::Error>(())
    });
  }

  Ok(())
}

fn ssl_route(_ctx: &RequestContext) -> HumptyResult<Response> {
  println!("SSL route called!");
  Ok(Response::ok(ResponseBody::from_slice("Its all good man!"), MimeType::TextPlain))
}
