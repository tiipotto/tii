use crate::mock_stream::MockStream;
use std::sync::Mutex;
use tii::TiiBuilder;
use tii::TiiMimeType;
use tii::TiiRequestContext;
use tii::TiiResponse;
use tii::TiiResult;

mod mock_stream;

static REQ_ID: Mutex<u128> = Mutex::new(0);
fn dummy_route(ctx: &TiiRequestContext) -> TiiResult<TiiResponse> {
  *REQ_ID.lock().unwrap() = ctx.id();

  TiiResponse::ok(format!("{:?}", ctx), TiiMimeType::TextPlain).into()
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
  let len = id.to_string().len() + 669; //The decimal len of the id is not padded and has a variable len.

  let raw = r#", peer_address: "Box", local_address: "Box", request: TiiRequestHead { method: Get, version: Http11, status_line: "GET /dummy HTTP/1.1", path: "/dummy", query: [], accept: [TiiAcceptQualityMimeType { value: Wildcard, q: TiiQValue(1000) }], content_type: None, headers: Headers([TiiHttpHeader { name: Connection, value: "Keep-Alive" }, TiiHttpHeader { name: TransferEncoding, value: "chunked" }]) }, body: Some(TiiRequestBody(Mutex { data: Chunked(RequestBodyChunked(eof=false remaining_chunk_length=0)), poisoned: false, .. })), force_connection_close: false, stream_meta: None, routed_path: Some("/dummy"), path_params: None, properties: None }"#;
  let expected_data = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: {len}\r\n\r\nTiiRequestContext {{ id: {id}{raw}");
  //Hint: this assert will obviously fail if we change the data structure of RequestContext or RequestHead. Just adjust the test in this case.
  assert_eq!(data, expected_data);
}
