use tii::extras::{TiiConnector, TiiTcpConnector};
use tii::TiiBuilder;
use tii::TiiMimeType;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::TiiResult;

const HTML: &str = r##"
<html>

<head>
  <title>Tii Wildcard Example</title>

  <script>
    function goToWildcard() {
      let text = document.querySelector("#text").value;
      window.location = `/wildcard/${text}`;
    }
  </script>
</head>

<body>
  <h1>Tii Wildcard Example</h1>

  Type anything in the box below and press the button.
  <br><br>

  <input id="text" placeholder="Type something here">
  <button onclick="goToWildcard();">Go to wildcard page</button>
</body>

</html>"##;

fn main() -> TiiResult<()> {
  let tii_server = TiiBuilder::builder_arc(|builder| {
    builder.router(|router| router.route_any("/", home)?.route_any("/wildcard/*", wildcard))
  })?;

  let _ = TiiTcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?.join(None);

  Ok(())
}

fn home(_: &TiiRequestContext) -> TiiResult<TiiResponse> {
  Ok(TiiResponse::ok(HTML, TiiMimeType::TextHtml))
}

fn wildcard(request: &TiiRequestContext) -> TiiResult<TiiResponse> {
  let wildcard_path = request
    .request_head()
    .get_path() // get the URI of the request
    .strip_prefix("/wildcard/") // remove the initial slash
    .unwrap(); // unwrap from the option

  let html = format!("<html><body><h1>Wildcard Path: {}</h1></body></html>", wildcard_path);

  Ok(TiiResponse::ok(html, TiiMimeType::TextHtml))
}
