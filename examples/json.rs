use std::fmt::{Debug, Formatter};
use std::io;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::time::Duration;
use log::{info, LevelFilter};
use serde::{Deserialize, Serialize};
use tii::{configure_type_system, MimeType, RequestContext, Response, ResponseBody, ResponseContext, Serializer, ServerBuilder, TiiError, TiiResult, UserError};
use tii::extras::{Connector, TcpConnector};

/// Serializer, it may do whatever you want it to do, in this case we use serde to create json.
/// You could make xml, plain text, yaml...
fn json<T: Serialize>(mime: &MimeType, data: T) -> TiiResult<Vec<u8>> {
    if &MimeType::ApplicationJson != mime {
        Err(io::Error::new(ErrorKind::InvalidInput, format!("Only application/json mime type is supported got {mime}")))?
    }
    Ok(serde_json::to_vec(&data)?)
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
    let human = Human {
        date_of_birth: "27.11.1988".to_string(),
        name: "Max Mustermann".to_string(),
    };
    Response::ok(ResponseBody::from_entity(human, json), MimeType::ApplicationJson)
}

fn get_house(_: &RequestContext) -> Response {
    let house = House {
        address: "Musterstraße 5, 10453 Musterstadt".to_string(),
        construction_date: "10.03.1974".to_string(),
    };
    Response::ok(ResponseBody::from_entity(house, json), MimeType::ApplicationJson)
}

fn get_cat(_: &RequestContext) -> Response {
    let cat = Cat {
        date_of_birth: "12.10.2022".to_string(),
        name: "Gustav".to_string(),
        species: "Persian".to_string(),
    };
    Response::ok(ResponseBody::from_entity(cat, json), MimeType::ApplicationJson)
}

/////////////////////////// RESPONSE FILTERS //////////////////////////////////

fn debug_responses_filter(resp: &mut ResponseContext) -> TiiResult<()> {
    _= resp.cast_response_entity::<dyn Debug, _>(|entity| {
        info!("Entity={:?}", entity);
    });
    Ok(())
}

fn redact_date_of_birth_filter(resp: &mut ResponseContext) -> TiiResult<()> {
    _= resp.cast_response_entity_mut::<dyn HasDateOfBirth, _>(|entity| {
        info!("Redacting Date of Birth {}", entity.get_date_of_birth());
        entity.set_date_of_birth("**Redacted**".to_string());
    });
    Ok(())
}

//////////////////// INIT AND SERVER ////////////////////////////////////////
fn main() -> TiiResult<()>{
    trivial_log::init_stdout(LevelFilter::Info).unwrap();

    let tii_server = ServerBuilder::builder_arc(|builder| {
        builder
            .router(|router| router
                .route_get("/human", get_human)?
                .route_get("/cat", get_cat)?
                .route_get("/house", get_house)?
                .with_response_filter(debug_responses_filter)?
                .with_response_filter(redact_date_of_birth_filter)?
                .with_response_filter(debug_responses_filter)
            )?
            .type_system(|type_system| {
                //We have to tell tii which entity structs impl which Traits.
                //It is by no means an error to "not" list a trait or struct here
                //However a filter which for example wants to filter out all HasDateOfBirth
                //structs will not see structs that don't have the type configured.
                configure_type_system!(type_system, Human, HasDateOfBirth, Debug);
                configure_type_system!(type_system, Cat, HasDateOfBirth, Debug);
                configure_type_system!(type_system, House, Debug);
            })
            .with_keep_alive_timeout(Some(Duration::ZERO)) //We disable http keep alive.
    })
        .unwrap();

    //curl -v http://localhost:8080/house
    //curl -v http://localhost:8080/cat
    //curl -v http://localhost:8080/human
    let _ = TcpConnector::start_unpooled("0.0.0.0:8080", tii_server)?.join(None);
    Ok(())
}
