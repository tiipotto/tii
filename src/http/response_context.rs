use crate::{RequestContext, Response, TypeSystemError};
use std::any::Any;
use std::mem;

/// Context object for response filters
/// Contains both info of the request and response
#[derive(Debug)]
pub struct ResponseContext<'request> {
  request: &'request mut RequestContext,
  response: Response,
}

impl<'a> ResponseContext<'a> {
  /// Constructor
  pub fn new(request: &'a mut RequestContext, response: Response) -> ResponseContext<'a> {
    Self { request, response }
  }

  /// Get all info from request
  pub fn get_request(&self) -> &RequestContext {
    self.request
  }

  /// Get all info from response
  pub fn get_response(&self) -> &Response {
    &self.response
  }

  /// Get mutable request info
  pub fn get_request_mut(&mut self) -> &mut RequestContext {
    self.request
  }

  /// Get mutable response info
  pub fn get_response_mut(&mut self) -> &mut Response {
    &mut self.response
  }

  /// Completely replaces the response
  pub fn set_response(&mut self, response: Response) -> Response {
    mem::replace(&mut self.response, response)
  }

  /// Casts the response entity (if the response contains one) to the desired type and calls
  /// a closure with it
  /// # Errors
  /// if casting fails because the response entity is not of the correct type.
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

  /// Casts the mutable response entity (if the response contains one) to the desired type and calls
  /// a closure with it
  /// # Errors
  /// if casting fails because the response entity is not of the correct type.
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

  /// Separates response and request info from each other.
  pub fn unwrap(self) -> (&'a RequestContext, Response) {
    (self.request, self.response)
  }
}
