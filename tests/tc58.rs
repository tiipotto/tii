use crate::mock_stream::MockStream;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;
use tii::{configure_type_system, MimeType, RequestBody, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;

fn to_json<T: Serialize>(mime: &MimeType, data: T) -> TiiResult<Vec<u8>> {
  if &MimeType::ApplicationJson != mime {
    Err(io::Error::new(
      ErrorKind::InvalidInput,
      format!("Only application/json mime type is supported got {mime}"),
    ))?
  }
  Ok(serde_json::to_vec(&data)?)
}

fn from_json<T: DeserializeOwned>(mime: &MimeType, data: &RequestBody) -> TiiResult<T> {
  if &MimeType::ApplicationJson != mime {
    Err(io::Error::new(
      ErrorKind::InvalidInput,
      format!("Only application/json mime type is supported got {mime}"),
    ))?
  }
  let data = data.read_to_vec()?;
  Ok(serde_json::from_reader::<_, T>(data.as_slice())?)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct FakeEntity {
  d1: i64,
  d2: i64,
}

fn dummy_filter(ctx: &mut RequestContext) {
  ctx
    .cast_request_entity::<FakeEntity, _>(|entity| {
      assert_eq!(entity.d1, 1);
      assert_eq!(entity.d2, 2);
      true
    })
    .unwrap();

  ctx
    .cast_request_entity::<dyn Debug, _>(|entity| {
      let data = format!("{entity:?}");
      assert_eq!("FakeEntity { d1: 1, d2: 2 }", data);
      true
    })
    .unwrap();

  ctx.set_property("beep", "bop");
}

fn dummy_route(ctx: &RequestContext, entity: &FakeEntity) -> TiiResult<Response> {
  let v = ctx.get_property::<&str>("beep").unwrap();
  assert_eq!(*v, "bop");

  assert_eq!(entity.d1, 1);
  assert_eq!(entity.d2, 2);
  Ok(Response::ok_entity(FakeEntity { d1: 3, d2: 4 }, to_json, MimeType::ApplicationJson))
}
#[test]
pub fn tc58() {
  let server = ServerBuilder::default()
    .router(|rt| {
      rt.put("/*")
        .consumes(MimeType::ApplicationJson)
        .produces(MimeType::ApplicationJson)
        .entity_endpoint(dummy_route, from_json)?
        .with_request_filter(dummy_filter)
    })
    .expect("ERR")
    .type_system(|type_sys| {
      configure_type_system!(type_sys, FakeEntity, Debug);
    })
    .build();

  let stream = MockStream::with_str("PUT /dummy HTTP/1.1\r\nConnection: keep-alive\r\nContent-Type: application/json\r\nContent-Length: 45\r\n\r\n{\"d1\":1,\"d2\":2}");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let got = String::from_utf8(stream.copy_written_data()).unwrap();

  assert_eq!(got.as_str(), "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: Keep-Alive\r\nContent-Length: 15\r\n\r\n{\"d1\":3,\"d2\":4}");
}
