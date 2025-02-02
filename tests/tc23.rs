use crate::mock_stream::MockStream;
use std::sync::Mutex;
use tii::http::mime::MimeType;
use tii::http::request_context::RequestContext;
use tii::http::Response;
use tii::tii_builder::TiiBuilder;
use tii::tii_error::TiiResult;

mod mock_stream;

static REQ_ID: Mutex<u128> = Mutex::new(0);
fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  *REQ_ID.lock().unwrap() = ctx.id();

  Response::ok(format!("{:?}", ctx), MimeType::TextPlain).into()
}

#[test]
pub fn tc23() {
  let server =
    TiiBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str("GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n");
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  let id = *REQ_ID.lock().unwrap();
  let len = id.to_string().len() + 640; //The decimal len of the id is not padded and has a variable len.

  let raw = r#", peer_address: "Box", local_address: "Box", request: RequestHead { method: Get, version: Http11, status_line: "GET /dummy HTTP/1.1", path: "/dummy", query: [], accept: [AcceptQualityMimeType { value: Wildcard, q: QValue(1000) }], content_type: None, headers: Headers([Header { name: Connection, value: "Keep-Alive" }, Header { name: TransferEncoding, value: "chunked" }]) }, body: Some(RequestBody(Mutex { data: Chunked(RequestBodyChunked(eof=false remaining_chunk_length=0)), poisoned: false, .. })), force_connection_close: false, stream_meta: None, routed_path: Some("/dummy"), path_params: None, properties: None }"#;
  let expected_data = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: {len}\r\n\r\nRequestContext {{ id: {id}{raw}");
  //Hint: this assert will obviously fail if we change the data structure of RequestContext or RequestHead. Just adjust the test in this case.
  assert_eq!(data, expected_data);
}
