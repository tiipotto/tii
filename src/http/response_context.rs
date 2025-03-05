use crate::{RequestContext, Response, TypeSystemError};
use std::any::Any;

#[derive(Debug)]
pub struct ResponseContext<'request> {
  request: &'request mut RequestContext,
  response: Response,
}

impl<'a> ResponseContext<'a> {
  pub fn new(request: &'a mut RequestContext, response: Response) -> ResponseContext<'a> {
    Self { request, response }
  }
  pub fn get_request(&self) -> &RequestContext {
    &self.request
  }

  pub fn get_response(&self) -> &Response {
    &self.response
  }

  pub fn get_request_mut(&mut self) -> &mut RequestContext {
    &mut self.request
  }

  pub fn get_response_mut(&mut self) -> &mut Response {
    &mut self.response
  }

  pub fn set_response(&mut self, response: Response) {
    self.response = response;
  }

  pub fn cast_response_entity<DST: Any + ?Sized + 'static, RET: Any + 'static>(
    &self,
    receiver: impl FnOnce(&DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    self
      .response
      .get_body()
      .ok_or(TypeSystemError::SourceTypeUnknown)?
      .entity_cast::<DST, RET>(self.request.get_type_system(), receiver)
  }
  pub fn cast_response_entity_mut<DST: Any + ?Sized + 'static, RET: Any + 'static>(
    &mut self,
    receiver: impl FnOnce(&mut DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    self
      .response
      .get_body_mut()
      .ok_or(TypeSystemError::SourceTypeUnknown)?
      .entity_cast_mut::<DST, RET>(self.request.get_type_system(), receiver)
  }

  pub fn unwrap(self) -> (&'a RequestContext, Response) {
    (self.request, self.response)
  }
}
