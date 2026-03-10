use crate::mock_stream::MockStream;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;
use tii::{configure_type_system, MimeType, MimeTypeWithCharset, RequestBody, TiiResult};
use tii::{RequestContext, Response, ServerBuilder};

mod mock_stream;

fn to_json<T: Serialize>(mime: &MimeTypeWithCharset, data: T) -> TiiResult<Vec<u8>> {
  if &MimeType::ApplicationJson != mime.mime() {
    Err(io::Error::new(
      ErrorKind::InvalidInput,
      format!("Only application/json mime type is supported got {mime}"),
    ))?
  }

  Ok(serde_json::to_vec(&data)?)
}

fn from_json<T: DeserializeOwned>(mime: &MimeTypeWithCharset, data: &RequestBody) -> TiiResult<T> {
  if &MimeType::ApplicationJson != mime.mime() {
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

struct DummyState {
  state: u32,
}

impl DummyState {
  fn dummy_route(&self, ctx: &RequestContext, entity: &FakeEntity) -> TiiResult<Response> {
    assert_eq!(self.state, 5);
    let v = ctx.get_property::<&str>("beep").unwrap();
    assert_eq!(*v, "bop");

    assert_eq!(entity.d1, 1);
    assert_eq!(entity.d2, 2);
    Ok(Response::ok_entity(FakeEntity { d1: 3, d2: 4 }, to_json, MimeType::ApplicationJson))
  }

  fn dummy_route2(&self, ctx: &RequestContext, entity: &FakeEntity) -> TiiResult<Response> {
    assert_eq!(self.state, 5);
    let v = ctx.get_property::<&str>("beep").unwrap();
    assert_eq!(*v, "bop");

    assert_eq!(entity.d1, 4);
    assert_eq!(entity.d2, 2);
    Ok(Response::ok_entity(FakeEntity { d1: 5, d2: 6 }, to_json, MimeType::ApplicationJson))
  }

  fn dummy_filter(&self, ctx: &mut RequestContext) {
    assert_eq!(self.state, 5);

    if ctx.get_path() == "/dummy1" {
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
    }

    if ctx.get_path() == "/dummy2" {
      ctx
        .cast_request_entity::<FakeEntity, _>(|entity| {
          assert_eq!(entity.d1, 4);
          assert_eq!(entity.d2, 2);
          true
        })
        .unwrap();

      ctx
        .cast_request_entity::<dyn Debug, _>(|entity| {
          let data = format!("{entity:?}");
          assert_eq!("FakeEntity { d1: 4, d2: 2 }", data);
          true
        })
        .unwrap();
    }

    ctx.set_property("beep", "bop");
  }
}
#[test]
pub fn tc61() {
  let state = DummyState { state: 5 };

  let my_state: &'static DummyState = Box::leak(Box::new(state));

  let server = ServerBuilder::default()
    .router(|rt| {
      rt.put("/dummy1")
        .consumes(MimeType::ApplicationJson)
        .produces(MimeType::ApplicationJson)
        .stateful_entity_endpoint(my_state, DummyState::dummy_route, from_json)?
        .put("/dummy2")
        .consumes(MimeType::ApplicationJson)
        .produces(MimeType::ApplicationJson)
        .stateful_entity_endpoint(my_state, DummyState::dummy_route2, from_json)?
        .with_request_filter((my_state, DummyState::dummy_filter))
    })
    .expect("ERR")
    .type_system(|type_sys| {
      configure_type_system!(type_sys, FakeEntity, Debug);
    })
    .build();

  let stream = MockStream::with_str("PUT /dummy1 HTTP/1.1\r\nConnection: keep-alive\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"d1\":1,\"d2\":2}PUT /dummy2 HTTP/1.1\r\nConnection: keep-alive\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"d1\":4,\"d2\":2}");
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let got = String::from_utf8(stream.copy_written_data()).unwrap();

  assert_eq!(got.as_str(), "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: Keep-Alive\r\nContent-Length: 15\r\n\r\n{\"d1\":3,\"d2\":4}HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: Keep-Alive\r\nContent-Length: 15\r\n\r\n{\"d1\":5,\"d2\":6}");
}
