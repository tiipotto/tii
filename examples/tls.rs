use log::info;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::io::{BufReader, Cursor};
use std::sync::Arc;
use tii::extras;
use tii::extras::{Connector, ConnectorMeta};
use tii::MimeType;
use tii::RequestContext;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiResult;

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

fn create_rust_tls_server_config() -> Arc<ServerConfig> {
  let certs = load_certs();
  let key = load_private_key();

  let config =
    ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key).expect("Error");
  Arc::new(config)
}

fn main() -> TiiResult<()> {
  colog::default_builder().filter_level(log::LevelFilter::Debug).init();

  let app =
    ServerBuilder::builder_arc(|builder| builder.router(|r| r.route_any("/tls", tls_route)))?;
  let config = create_rust_tls_server_config();

  //Non Tls connectors

  //curl -v http://localhost:8080/tls
  let _tcp = extras::TcpConnector::start_unpooled("0.0.0.0:8080", app.clone())?;

  //curl -v --unix-socket /tmp/tii.sock http://localhost:8080/tls
  #[cfg(unix)]
  let _unix = extras::UnixConnector::start_unpooled("/tmp/tii.sock", app.clone())?;

  // TLS connectors

  //curl -k -v --unix-socket /tmp/tiitls.sock https://localhost:8443/tls
  #[cfg(unix)]
  let _unix_tls =
    extras::TlsUnixConnector::start_unpooled("/tmp/tiitls.sock", config.clone(), app.clone())?;

  //curl -k -v https://localhost:8443/tls
  extras::TlsTcpConnector::start_unpooled("0.0.0.0:8443", config, app)?.join(None);

  Ok(())
}

fn tls_route(ctx: &RequestContext) -> Response {
  info!("/tls route called");

  match ctx.get_stream_meta::<ConnectorMeta>() {
    Some(meta) => match meta {
      ConnectorMeta::TlsTcp => Response::ok("Tls Connection via Tcp socket", MimeType::TextPlain),
      #[cfg(unix)]
      ConnectorMeta::TlsUnix => Response::ok("Tls Connection via Unix socket", MimeType::TextPlain),
      ConnectorMeta::Tcp => {
        Response::forbidden("Plain text Connection via Tcp socket", MimeType::TextPlain)
      }

      #[cfg(unix)]
      ConnectorMeta::Unix => {
        Response::forbidden("Plain text Connection via Unix socket", MimeType::TextPlain)
      }

      _ => Response::forbidden(format!("Connection type {meta} is not known"), MimeType::TextPlain),
    },
    None => Response::forbidden("Connection type not known", MimeType::TextPlain),
  }
}
