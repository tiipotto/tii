use crate::mock_stream::MockStream;
use std::sync::Mutex;
use tii::MimeType;
use tii::RequestContext;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiResult;

mod mock_stream;

static REQ_ID: Mutex<u128> = Mutex::new(0);
static REQ_TSP: Mutex<u128> = Mutex::new(0);

fn dummy_route(ctx: &RequestContext) -> TiiResult<Response> {
  *REQ_ID.lock().unwrap() = ctx.id();
  *REQ_TSP.lock().unwrap() = ctx.get_timestamp();

  Response::ok(format!("{ctx:?}"), MimeType::TextPlain).into()
}

#[test]
pub fn tc23() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/dummy", dummy_route)).expect("ERR").build();

  let stream = MockStream::with_str(
    "GET /dummy HTTP/1.1\r\nConnection: Keep-Alive\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n10\r\n1234567890123456\r\n0\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).expect("ERROR");
  let data = stream.copy_written_data_to_string();
  let id = *REQ_ID.lock().unwrap();
  let tsp = *REQ_TSP.lock().unwrap();
  let len = id.to_string().len() + tsp.to_string().len() + 756; //The decimal len of the id is not padded and has a variable len.

  let raw = r#", peer_address: "Box", local_address: "Box", request: RequestHead { method: Get, version: Http11, status_line: "GET /dummy HTTP/1.1", path: "/dummy", query: [], accept: [AcceptQualityMimeType { value: Wildcard, q: QValue(1000) }], content_type: None, headers: Headers([HttpHeader { name: Connection, value: "Keep-Alive" }, HttpHeader { name: TransferEncoding, value: "chunked" }]) }, body: Some(RequestBody(Mutex { data: Chunked(RequestBodyChunked(eof=false remaining_chunk_length=0)), poisoned: false, .. })), request_entity: None, force_connection_close: false, stream_meta: None, routed_path: Some("/dummy"), path_params: None, properties: None, type_system: TypeSystem(TypeSystemBuilder { types: {}, types_mut: {} }) }"#;
  let expected_data = format!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Keep-Alive\r\nContent-Length: {len}\r\n\r\nRequestContext {{ id: {id}, timestamp: {tsp}{raw}"
  );
  //Hint: this assert will obviously fail if we change the data structure of RequestContext or RequestHead. Just adjust the test in this case.
  assert_eq!(data, expected_data);
}
