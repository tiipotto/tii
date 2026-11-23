//! A long-running JSON-RPC call that reports progress while it runs, shaped like an MCP tool call
//! over Streamable HTTP. The response is a `text/event-stream` with one `notifications/progress`
//! event per symbol, then the result. Each event is flushed, so the client receives it as soon as
//! it is written instead of when the call finishes.
//!
//! ```bash
//! curl -N http://localhost:8080/mcp -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lookup","arguments":{"symbols":["AAPL","MSFT","NVDA"]},"_meta":{"progressToken":"lookup-1"}}}'
//! ```
use serde_json::{Value, json};
use std::thread::sleep;
use std::time::Duration;
use tii::extras::{Connector, TcpConnector};
use tii::{MimeType, RequestContext, Response, ResponseBody, ServerBuilder, TiiResult};

fn tools_call(request: &RequestContext) -> TiiResult<Response> {
  let body = match request.request_body() {
    Some(body) => body.read_to_vec()?,
    None => Vec::new(),
  };
  let Ok(call) = serde_json::from_slice::<Value>(&body) else {
    return Ok(Response::bad_request("expected a JSON-RPC request", MimeType::TextPlain));
  };
  let id = call["id"].clone();
  let token = call["params"]["_meta"]["progressToken"].clone();
  let symbols: Vec<String> =
    serde_json::from_value(call["params"]["arguments"]["symbols"].clone()).unwrap_or_default();

  let events = ResponseBody::chunked(move |sink| {
    let send = |message: Value| {
      sink.write_all(format!("data: {message}\n\n").as_bytes())?;
      sink.flush()
    };
    // Sends the response headers right away, before the first step finishes.
    sink.flush()?;
    for (done, symbol) in symbols.iter().enumerate() {
      // The slow part of the tool, such as a web request per symbol.
      sleep(Duration::from_secs(1));
      if !token.is_null() {
        send(json!({
          "jsonrpc": "2.0",
          "method": "notifications/progress",
          "params": {
            "progressToken": token,
            "progress": done + 1,
            "total": symbols.len(),
            "message": format!("looked up {symbol}"),
          },
        }))?;
      }
    }
    send(json!({
      "jsonrpc": "2.0",
      "id": id,
      "result": {"content": [{"type": "text", "text": format!("looked up {}", symbols.join(", "))}]},
    }))
  });
  Ok(Response::ok(events, MimeType::parse("text/event-stream").expect("valid mime type")))
}

fn main() {
  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder.router(|router| router.route_post("/mcp", tools_call)?.ok())
  })
  .unwrap();

  let _ = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server).unwrap().join(None);
}
