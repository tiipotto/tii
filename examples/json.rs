//! Example that showcases json processing of a typical Restful web service.
//!
//! It sows how request and response bodies can be parsed to structured data using serde
//! (or any other serialization mechanism of your choice)
//! as well as showing how filters can access the structured data directly or
//! as a dyn compatible trait that the structs implement.
//!
//! We have a bunch of endpoints which return json data as response bodies.
//! 2 of 3 implement a trait called HasDateOfBirth.
//! There is a filter that redacts the date of birth because it's meant to be hidden in this example.
//! In addition, there is a filter that logs the response entity before and after the redaction so you can see the difference.
//!
//! There is also 1 PUT endpoint which accepts a json entity Cat. Cat has a field called species for which we
//! do not want to permit the value `Manul` in this example.
//! There is a request filter for that which will abort the request with http 403 forbidden
//! without the endpoint being called if the species contains `Manul`.
//!
//! The relevant curl commands to invoke all 'cases' of this example are as follows:
//! ```bash
//! # Date of birth will not be filtered because House does not implement the trait HasDateOfBirth
//! curl -v http://localhost:8080/house
//!
//! # Date of birth will be filtered out
//! curl -v http://localhost:8080/cat
//!
//! # Date of birth will be filtered out
//! curl -v http://localhost:8080/human
//!
//! # PUT will succeed
//! curl -v -X PUT -d '{"date_of_birth": "01.01.2023", "species":"Bengal", "name": "Bob"}' -H "Content-Type: application/json" http://localhost:8080/cat
//!
//! # PUT will fail because the species is Manul
//! curl -v -X PUT -d '{"date_of_birth": "04.05.2024", "species":"Manul", "name": "Rob"}' -H "Content-Type: application/json" http://localhost:8080/cat
//! ```
//!
use log::{info, LevelFilter};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;
use tii::extras::{Connector, TcpConnector};
use tii::{
  configure_type_system, MimeType, RequestBody, RequestContext, Response, ResponseBody,
  ResponseContext, ServerBuilder, TiiResult,
};

/// Serializer, it may do whatever you want it to do, in this case we use serde to create json.
/// You could make xml, plain text, yaml...
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

///////////////// ENTITIES / RESULT MODEL //////////////////////
#[derive(Serialize, Deserialize, Debug)]
struct Cat {
  date_of_birth: String,
  name: String,
  species: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Human {
  date_of_birth: String,
  name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct House {
  address: String,
  construction_date: String,
}

///////////////// TRAITS FOR SOME RESULT MODELS //////////////////////

trait HasDateOfBirth {
  fn get_date_of_birth(&self) -> &str;
  fn set_date_of_birth(&mut self, date_of_birth: String);
}

impl HasDateOfBirth for Human {
  fn get_date_of_birth(&self) -> &str {
    self.date_of_birth.as_ref()
  }
  fn set_date_of_birth(&mut self, date_of_birth: String) {
    self.date_of_birth = date_of_birth;
  }
}

impl HasDateOfBirth for Cat {
  fn get_date_of_birth(&self) -> &str {
    self.date_of_birth.as_ref()
  }

  fn set_date_of_birth(&mut self, date_of_birth: String) {
    self.date_of_birth = date_of_birth;
  }
}

/////////////////////// ENDPOINTS ////////////////////////////

fn get_human(_: &RequestContext) -> Response {
  let human = Human { date_of_birth: "27.11.1988".to_string(), name: "Max Mustermann".to_string() };
  Response::ok(ResponseBody::from_entity(human, to_json), MimeType::ApplicationJson)
}

fn get_house(_: &RequestContext) -> Response {
  let house = House {
    address: "Musterstraße 5, 10453 Musterstadt".to_string(),
    construction_date: "10.03.1974".to_string(),
  };
  Response::ok(ResponseBody::from_entity(house, to_json), MimeType::ApplicationJson)
}

fn get_cat(_: &RequestContext) -> Response {
  let cat = Cat {
    date_of_birth: "12.10.2022".to_string(),
    name: "Gustav".to_string(),
    species: "Persian".to_string(),
  };
  Response::ok(ResponseBody::from_entity(cat, to_json), MimeType::ApplicationJson)
}

fn put_cat(_: &RequestContext, cat: &Cat) -> Response {
  info!("Putting cat {:?}", cat);
  Response::no_content()
}

/////////////////////////// REQUEST FILTERS ///////////////////////////////////

fn block_manul_cats(req: &mut RequestContext) -> TiiResult<Option<Response>> {
  let is_manul = req.cast_request_entity::<Cat, _>(|cat| cat.species == "Manul").unwrap_or(false);

  if is_manul {
    return Ok(Some(Response::forbidden("Manuls are not pets!", MimeType::TextPlain)));
  }

  Ok(None)
}

/////////////////////////// RESPONSE FILTERS //////////////////////////////////

fn debug_responses_filter(resp: &mut ResponseContext<'_>) -> TiiResult<()> {
  _ = resp.cast_response_entity::<dyn Debug, _>(|entity| {
    info!("Response Entity={:?}", entity);
  });
  _ = resp.get_request().cast_request_entity::<dyn Debug, _>(|entity| {
    info!("Request Entity={:?}", entity);
  });
  Ok(())
}

fn redact_date_of_birth_filter(resp: &mut ResponseContext<'_>) -> TiiResult<()> {
  _ = resp.cast_response_entity_mut::<dyn HasDateOfBirth, _>(|entity| {
    info!("Redacting Date of Birth {}", entity.get_date_of_birth());
    entity.set_date_of_birth("**Redacted**".to_string());
  });
  Ok(())
}

//////////////////// INIT AND SERVER ////////////////////////////////////////
fn main() -> TiiResult<()> {
  trivial_log::init_stdout(LevelFilter::Info).unwrap();

  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder
      .router(|router| {
        router
          .route_get("/human", get_human)?
          .route_get("/cat", get_cat)?
          .route_get("/house", get_house)?
          .begin_put("/cat", |route| {
            route.consumes(MimeType::ApplicationJson).entity_endpoint(put_cat, from_json)
          })?
          .with_request_filter(block_manul_cats)?
          .with_response_filter(debug_responses_filter)?
          .with_response_filter(redact_date_of_birth_filter)?
          .with_response_filter(debug_responses_filter)
      })?
      .type_system(|type_system| {
        //We have to tell tii which entity structs impl which Traits.
        //It is by no means an error to "not" list a trait or struct here
        //However a filter which for example wants to filter out all HasDateOfBirth
        //structs will not see structs that don't have the type configured.
        configure_type_system!(type_system, Human, HasDateOfBirth, Debug);
        configure_type_system!(type_system, Cat, HasDateOfBirth, Debug);
        configure_type_system!(type_system, House, Debug);
      })
      .ok()
  })
  .unwrap();

  let _ = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?.join(None);
  Ok(())
}
