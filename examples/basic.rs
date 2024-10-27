use humpty::http::request_context::RequestContext;
use humpty::http::{Response, StatusCode};
use humpty::humpty_builder::HumptyBuilder;
use std::error::Error;
use std::net::TcpListener;
use std::{io, thread};
use humpty::humpty_error::HumptyResult;

fn main() -> Result<(), Box<dyn Error>> {
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
  println!("pre_routing {:?}", req);
  Ok(None)
}

fn routing(req: &mut RequestContext) -> HumptyResult<Option<Response>> {
  println!("routing {:?}", req);
  Ok(None)
}

fn resp(req: &mut RequestContext, mut resp: Response) -> HumptyResult<Response> {
  println!("resp {:?}", req);
  resp.headers.add("X-Magic", "true magic");
  Ok(resp)
}

fn home(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::new(StatusCode::OK, "<html><body><h1>Home</h1></body></html>"))
}

fn contact(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::new(StatusCode::OK, "<html><body><h1>Contact</h1></body></html>"))
}

fn generic(request: &RequestContext) -> HumptyResult<Response> {
  let html = format!(
    "<html><body><h1>You just requested {}.</h1></body></html>",
    request.request_head().path
  );

  Ok(Response::new(StatusCode::OK, html))
}

fn pong(request: &RequestContext) -> HumptyResult<Response> {
  let body = if let Some(body) = request.request_body() {
    let mut buffer = Vec::new();
    body.clone().read_to_end(&mut buffer)?;
    buffer
  } else {
    b"No Body".to_vec()
  };

  Ok(Response::new(StatusCode::OK, body))
}
