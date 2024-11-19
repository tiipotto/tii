use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use log::info;
use std::error::Error;
use std::net::TcpListener;
use std::{io, thread};

fn main() -> Result<(), Box<dyn Error>> {
  colog::default_builder().filter_level(log::LevelFilter::Trace).init();

  let app = HumptyBuilder::default()
    .router(|router| {
      router
        .with_route("/", home)
        .with_route("/contact", contact)
        .with_route("/ping", pong)
        .with_route("/*", generic)
        .with_pre_routing_request_filter(pre_routing)
        .with_request_filter(routing)
        .with_response_filter(resp)
    })
    .build_arc();

  let listen = TcpListener::bind("0.0.0.0:8080")?;
  for stream in listen.incoming() {
    let app = app.clone();
    thread::spawn(move || {
      app.handle_connection(stream?).expect("ERORR");
      Ok::<(), io::Error>(())
    });
  }

  Ok(())
}

fn pre_routing(req: &mut RequestContext) -> HumptyResult<Option<Response>> {
  info!("pre_routing {:?}", req);
  Ok(None)
}

fn routing(req: &mut RequestContext) -> HumptyResult<Option<Response>> {
  info!("routing {:?}", req);
  Ok(None)
}

fn resp(req: &mut RequestContext, mut resp: Response) -> HumptyResult<Response> {
  info!("resp {:?}", req);
  resp.add_header("X-Magic", "true magic")?;
  Ok(resp)
}

fn home(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::ok("<html><body><h1>Home</h1></body></html>", MimeType::TextHtml))
}

fn contact(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::ok("<html><body><h1>Contact</h1></body></html>", MimeType::TextHtml))
}

fn generic(request: &RequestContext) -> HumptyResult<Response> {
  let html = format!(
    "<html><body><h1>You just requested {}.</h1></body></html>",
    request.request_head().path()
  );

  Ok(Response::ok(html, MimeType::TextHtml))
}

fn pong(request: &RequestContext) -> HumptyResult<Response> {
  let body = if let Some(body) = request.request_body() {
    let mut buffer = Vec::new();
    body.clone().read_to_end(&mut buffer)?;
    buffer
  } else {
    b"No Body".to_vec()
  };

  Ok(Response::ok(body, MimeType::ApplicationOctetStream))
}
