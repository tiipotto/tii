use crate::{MimeType, TiiResult, TypeSystem, TypeSystemError};
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter, Write};

pub trait Serializer<T: Any + Debug + 'static>: Any {
  fn serialize(&self, mime: &MimeType, data: T) -> TiiResult<Vec<u8>>;
}

impl<F, T> Serializer<T> for F where
    T: Any+Debug+'static,
    F: Fn(&MimeType, T) -> TiiResult<Vec<u8>>+'static,
{
  fn serialize(&self, mime: &MimeType, data: T) -> TiiResult<Vec<u8>> {
    self(mime, data)
  }
}

trait DynResponseEntityInner: Debug {
  fn serialize(&mut self, mime: &MimeType) -> TiiResult<Vec<u8>>;
  fn get_serializer(&self) -> &dyn Any;
  fn get_serializer_mut(&mut self) -> &mut dyn Any;
  fn get_entity(&self) -> &dyn Any;
  fn get_entity_mut(&mut self) -> &mut dyn Any;
  fn take_inner(&mut self) -> (Box<dyn Any>, Box<dyn Any>);
}

struct ResponseEntityInner<T: Any + Debug + 'static> {
  entity: Option<T>,
  serializer: Option<Box<dyn Serializer<T>>>,
}

impl<T: Any + Debug + 'static> Debug for ResponseEntityInner<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.entity, f)
  }
}
impl<T: Any + Debug + 'static> DynResponseEntityInner for ResponseEntityInner<T> {
  fn serialize(&mut self, mime: &MimeType) -> TiiResult<Vec<u8>> {
    self.serializer.as_ref().unwrap().serialize(mime, self.entity.take().unwrap())
  }

  fn get_serializer(&self) -> &dyn Any {
    self.serializer.as_ref().unwrap() as &dyn Any
  }

  fn get_serializer_mut(&mut self) -> &mut dyn Any {
    self.serializer.as_mut().unwrap() as &mut dyn Any
  }

  fn get_entity(&self) -> &dyn Any {
    self.entity.as_ref().unwrap() as &dyn Any
  }

  fn get_entity_mut(&mut self) -> &mut dyn Any {
    self.entity.as_mut().unwrap() as &mut dyn Any
  }

  fn take_inner(&mut self) -> (Box<dyn Any>, Box<dyn Any>) {
    (
      Box::new(self.entity.take().unwrap()) as Box<dyn Any>,
      self.serializer.take().unwrap() as Box<dyn Any>,
    )
  }
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ResponseEntity(Box<dyn DynResponseEntityInner>);

impl ResponseEntity {
  pub fn new<T: Any + Debug + 'static>(
    entity: T,
    serializer: impl Serializer<T> + 'static,
  ) -> Self {
    Self(Box::new(ResponseEntityInner {
      entity: Some(entity),
      serializer: Some(Box::new(serializer) as Box<dyn Serializer<T>>),
    }) as Box<dyn DynResponseEntityInner>)
  }
  pub fn serialize(mut self, mime: &MimeType) -> TiiResult<Vec<u8>> {
    //We must consume! not consuming self will panic upon second call via dynamic dispatch!
    self.0.serialize(mime)
  }

  pub fn into_inner(mut self) -> (Box<dyn Any>, Box<dyn Any>) {
    //We must consume! not consuming self will panic upon second call via dynamic dispatch!
    self.0.take_inner()
  }

  pub fn get_serializer(&self) -> &dyn Any {
    self.0.get_serializer()
  }

  pub fn get_serializer_mut(&mut self) -> &mut dyn Any {
    self.0.get_serializer_mut()
  }

  pub fn get_entity(&self) -> &dyn Any {
    self.0.get_entity()
  }

  pub fn get_entity_mut(&mut self) -> &mut dyn Any {
    self.0.get_entity_mut()
  }
}
