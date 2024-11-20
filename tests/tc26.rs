use crate::mock_stream::MockStream;
use humpty::http::mime::MimeType;
use humpty::http::request::HttpVersion;
use humpty::http::request_context::RequestContext;
use humpty::http::response_body::ResponseBody;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;

mod mock_stream;

fn dummy_route(ctx: &RequestContext) -> HumptyResult<Response> {
  assert_eq!(HttpVersion::Http11, ctx.request_head().version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));

  Ok(Response::ok(ResponseBody::from_slice("Okay!"), MimeType::TextPlain))
}

fn dummy_route2(ctx: &RequestContext) -> HumptyResult<Response> {
  assert_eq!(HttpVersion::Http11, ctx.request_head().version());
  assert_eq!(ctx.request_head().get_header("Hdr"), Some("test"));

  Ok(Response::ok(ResponseBody::from_slice("\"Nice!\""), MimeType::ApplicationJson))
}

#[test]
pub fn tc26() {
  let server = HumptyBuilder::default()
    .router(|rt| {
      rt.get("/dummy")
        .produces(MimeType::TextPlain)
        .endpoint(dummy_route)
        .get("/dummy")
        .produces(MimeType::ApplicationJson)
        .endpoint(dummy_route2)
    })
    .build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain;q=0.7, application/json;q=0.6\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain;q=0.5, application/json;q=0.6\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: Close\r\nContent-Length: 7\r\n\r\n\"Nice!\"");

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/*;q=0.5, application/json;q=0.6\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: Close\r\nContent-Length: 7\r\n\r\n\"Nice!\"");

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain;q=0.7, application/*;q=0.6\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/plain;q=0.5, application/*;q=0.6\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: Close\r\nContent-Length: 7\r\n\r\n\"Nice!\"");

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: text/*;q=0.7, application/json;q=0.6\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");

  //It's not clear what to do, so in this case we pick the first endpoint!
  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nHdr: test\r\nAccept: */*\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nOkay!");
}
