use crate::mock_stream::MockStream;
use humpty::http::mime::MimeType;
use humpty::http::request_context::RequestContext;
use humpty::http::Response;
use humpty::humpty_builder::HumptyBuilder;
use humpty::humpty_error::HumptyResult;
use std::sync::Mutex;

mod mock_stream;

static REQ_ID: Mutex<u128> = Mutex::new(0);
fn dummy_route(ctx: &RequestContext) -> HumptyResult<Response> {
  *REQ_ID.lock().unwrap() = ctx.id();

  Response::ok(format!("{:?}", ctx), MimeType::TextPlain).into()
}

#[test]
pub fn tc23() {
  let server = HumptyBuilder::default().router(|rt| rt.with_route("/dummy", dummy_route)).build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  let id = *REQ_ID.lock().unwrap();
  let len = id.to_string().len() + 563; //The decimal len of the id is not padded and has a variable len.

  let raw = r#", address: "Box", request: RequestHead { method: Get, version: Http11, status_line: "GET /dummy HTTP/1.1", path: "/dummy", query: [], accept: [AcceptMime { value: Wildcard, q: QValue(1000) }], headers: Headers([Header { name: Connection, value: "Keep-Alive" }, Header { name: TransferEncoding, value: "chunked" }]) }, body: Some(RequestBody(Mutex { data: Chunked(RequestBodyChunked(eof=false remaining_chunk_length=0)), poisoned: false, .. })), force_connection_close: false, stream_meta: None, routed_path: Some("/dummy"), properties: None }"#;
  let expected_data = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: {len}\r\n\r\nRequestContext {{ id: {id}{raw}");
  //Hint: this assert will obviously fail if we change the data structure of RequestContext or RequestHead. Just adjust the test in this case.
  assert_eq!(data, expected_data);
}
