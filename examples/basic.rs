use log::info;
use tii::extras::{TiiConnector, TiiTcpConnector};
use tii::TiiBuilder;
use tii::TiiHttpMethod;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::TiiResult;
use tii::{TiiAcceptMimeType, TiiMimeType};

fn main() -> TiiResult<()> {
  colog::default_builder().filter_level(log::LevelFilter::Trace).init();

  let tii_server = TiiBuilder::builder_arc(|builder| {
    //This example only has 1 router, you could have several by just calling .router(...) again.
    builder.router(|router| {
      router
        // All these different ways of adding routes do the same thing, pick the one most "optically" pleasing for you.
        //
        // Build endpoint "closure" style. Causes an indent making it easier to spot.
        .begin_get("/", |route| {
          route
            //
            .produces(TiiMimeType::TextHtml)
            .produces(TiiMimeType::TextPlain)
            .endpoint(home)
        })?
        //
        //build endpoint directly without any indents.
        .get("/contact")
        .produces(TiiMimeType::TextHtml)
        .endpoint(contact)?
        // as you can see without this comment it would be hard to tell what belongs to which endpoint.
        .post("/ping")
        .consumes(TiiAcceptMimeType::Wildcard)
        .produces(TiiMimeType::ApplicationOctetStream)
        .endpoint(pong)?
        //If you do not desire any media type handling you can also use the "route" type of methods.
        // This endpoint is called for any normal http method and any media type.
        .route_any("/any/method", echo_method)?
        //Same but limited to http GET method
        .route_get("/only/get", echo_method)?
        // Tii also supports non-standard custom methods.
        .route_method(TiiHttpMethod::from("QUERY"), "/custom/stuff", echo_method)?
        // Begin is just a visual indent so you can group several other things together.
        // It does nothing else.
        .route_get("/path/param/{key1}/{key2}/{regex:.*}", path_param)?
        .begin(|router| {
          router
            //
            .get("/closure/*")
            //You don't have to pass a function pointer, if your endpoint is tiny you can also do it in a closure
            //You do have to explicitly write out "&RequestContext" tho otherwise rust gets confused.
            .endpoint(|ctx: &TiiRequestContext| {
              TiiResponse::ok(
                format!("This is a closure to {}!", ctx.request_head().get_path()),
                TiiMimeType::TextPlain,
              )
            })?
            .get("/*")
            .produces(TiiMimeType::TextHtml)
            .endpoint(generic)
        })?
        // There 3 are not endpoints, they are filters etc.
        .with_pre_routing_request_filter(pre_routing)?
        .with_request_filter(routing)?
        .with_response_filter(resp)?
        .ok()
    })
  })?;

  let _ = TiiTcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?.join(None);

  Ok(())
}

fn pre_routing(req: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>> {
  info!("pre_routing {:?}", req);
  Ok(None)
}

fn routing(req: &mut TiiRequestContext) -> TiiResult<Option<TiiResponse>> {
  info!("routing {:?}", req);
  Ok(None)
}

fn resp(req: &mut TiiRequestContext, mut resp: TiiResponse) -> TiiResult<TiiResponse> {
  info!("resp {:?}", req);
  resp.add_header("X-Magic", "true magic")?;
  Ok(resp)
}

fn home(_: &TiiRequestContext) -> TiiResult<TiiResponse> {
  Ok(TiiResponse::ok("<html><body><h1>Home</h1></body></html>", TiiMimeType::TextHtml))
}

fn contact(_: &TiiRequestContext) -> TiiResult<TiiResponse> {
  Ok(TiiResponse::ok("<html><body><h1>Contact</h1></body></html>", TiiMimeType::TextHtml))
}

fn generic(request: &TiiRequestContext) -> TiiResult<TiiResponse> {
  let html = format!(
    "<html><body><h1>You just requested {}.</h1></body></html>",
    request.request_head().get_path()
  );

  Ok(TiiResponse::ok(html, TiiMimeType::TextHtml))
}

fn pong(request: &TiiRequestContext) -> TiiResult<TiiResponse> {
  let body = if let Some(body) = request.request_body() {
    let mut buffer = Vec::new();
    body.clone().read_to_end(&mut buffer)?;
    buffer
  } else {
    b"No Body".to_vec()
  };

  Ok(TiiResponse::ok(body, TiiMimeType::ApplicationOctetStream))
}

fn echo_method(request: &TiiRequestContext) -> TiiResponse {
  TiiResponse::ok(request.request_head().get_method().as_str(), TiiMimeType::TextPlain)
}

fn path_param(request: &TiiRequestContext) -> TiiResponse {
  for (key, value) in request.get_path_params() {
    info!("path_param {} {}", key, value);
  }

  TiiResponse::no_content()
}
