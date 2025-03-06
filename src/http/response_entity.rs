use crate::util::unwrap_some;
use crate::{MimeType, TiiResult};
use std::any::Any;
use std::fmt::{Debug, Formatter};

/// Trait for serializing entities to some bytes.
pub trait EntitySerializer<T: Any + Debug + 'static>: Any + Send {
  /// Perform the serialization
  fn serialize(&self, mime: &MimeType, data: T) -> TiiResult<Vec<u8>>;
}

impl<F, T> EntitySerializer<T> for F
where
  T: Any + Debug + Send + 'static,
  F: Fn(&MimeType, T) -> TiiResult<Vec<u8>> + Send + 'static,
{
  fn serialize(&self, mime: &MimeType, data: T) -> TiiResult<Vec<u8>> {
    self(mime, data)
  }
}

trait DynResponseEntityInner: Debug + Send {
  fn serialize(&mut self, mime: &MimeType) -> TiiResult<Vec<u8>>;
  fn get_serializer(&self) -> &dyn Any;
  fn get_serializer_mut(&mut self) -> &mut dyn Any;
  fn get_entity(&self) -> &dyn Any;
  fn get_entity_mut(&mut self) -> &mut dyn Any;
  fn take_inner(&mut self) -> (Box<dyn Any>, Box<dyn Any>);
}

struct ResponseEntityInner<T: Any + Debug + Send + 'static> {
  entity: Option<T>,
  serializer: Option<Box<dyn EntitySerializer<T>>>,
}

impl<T: Any + Debug + Send + 'static> Debug for ResponseEntityInner<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.entity, f)
  }
}
impl<T: Any + Debug + Send + 'static> DynResponseEntityInner for ResponseEntityInner<T> {
  fn serialize(&mut self, mime: &MimeType) -> TiiResult<Vec<u8>> {
    unwrap_some(self.serializer.as_ref()).serialize(mime, unwrap_some(self.entity.take()))
  }

  fn get_serializer(&self) -> &dyn Any {
    unwrap_some(self.serializer.as_ref()) as &dyn Any
  }

  fn get_serializer_mut(&mut self) -> &mut dyn Any {
    unwrap_some(self.serializer.as_mut()) as &mut dyn Any
  }

  fn get_entity(&self) -> &dyn Any {
    unwrap_some(self.entity.as_ref()) as &dyn Any
  }

  fn get_entity_mut(&mut self) -> &mut dyn Any {
    unwrap_some(self.entity.as_mut()) as &mut dyn Any
  }

  fn take_inner(&mut self) -> (Box<dyn Any>, Box<dyn Any>) {
    (
      Box::new(unwrap_some(self.entity.take())) as Box<dyn Any>,
      unwrap_some(self.serializer.take()) as Box<dyn Any>,
    )
  }
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ResponseEntity(Box<dyn DynResponseEntityInner>);

impl ResponseEntity {
  pub fn new<T: Any + Send + Debug + 'static>(
    entity: T,
    serializer: impl EntitySerializer<T> + 'static,
  ) -> Self {
    Self(Box::new(ResponseEntityInner {
      entity: Some(entity),
      serializer: Some(Box::new(serializer) as Box<dyn EntitySerializer<T>>),
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
