use humpty::extras::{Connector, TcpConnector};
use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;

const HTML: &str = r##"
<html>

<head>
  <title>Humpty Wildcard Example</title>

  <script>
    function goToWildcard() {
      let text = document.querySelector("#text").value;
      window.location = `/wildcard/${text}`;
    }
  </script>
</head>

<body>
  <h1>Humpty Wildcard Example</h1>

  Type anything in the box below and press the button.
  <br><br>

  <input id="text" placeholder="Type something here">
  <button onclick="goToWildcard();">Go to wildcard page</button>
</body>

</html>"##;

fn main() -> HumptyResult<()> {
  let humpty_server = HumptyBuilder::builder_arc(|builder| {
    builder.router(|router| router.route_any("/", home)?.route_any("/wildcard/*", wildcard))
  })?;

  let _ = TcpConnector::start("0.0.0.0:8080", humpty_server)?.join(None);

  Ok(())
}

fn home(_: &RequestContext) -> HumptyResult<Response> {
  Ok(Response::ok(HTML, MimeType::TextHtml))
}

fn wildcard(request: &RequestContext) -> HumptyResult<Response> {
  let wildcard_path = request
    .request_head()
    .path() // get the URI of the request
    .strip_prefix("/wildcard/") // remove the initial slash
    .unwrap(); // unwrap from the option

  let html = format!("<html><body><h1>Wildcard Path: {}</h1></body></html>", wildcard_path);

  Ok(Response::ok(html, MimeType::TextHtml))
}
