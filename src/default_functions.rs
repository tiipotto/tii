use crate::TiiRequestContext;
use crate::{error_log, info_log};
use crate::{TiiError, TiiResult};
use crate::{TiiResponse, TiiStatusCode};
use crate::{TiiRouteable, TiiRoutingDecision};
use std::collections::HashSet;

pub(crate) fn default_pre_routing_filter(_request: &TiiRequestContext) -> TiiResult<bool> {
  Ok(true)
}

/// The default error handler for every Tii app.
/// This can be overridden by using the `with_error_handler` method when building the app.
pub(crate) fn default_error_handler(
  request: &mut TiiRequestContext,
  error: TiiError,
) -> TiiResult<TiiResponse> {
  error_log!(
    "Internal Server Error {} {} {:?}",
    &request.request_head().get_method(),
    request.request_head().get_path(),
    error
  );
  Ok(TiiResponse::new(TiiStatusCode::InternalServerError))
}

pub(crate) fn default_fallback_not_found_handler(
  request: &mut TiiRequestContext,
) -> TiiResult<TiiResponse> {
  info_log!(
    "Fallback: Not found {} {}",
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  Ok(TiiResponse::not_found_no_body())
}

pub(crate) fn default_not_found_handler(
  request: &mut TiiRequestContext,
  _: &[TiiRouteable],
) -> TiiResult<TiiResponse> {
  info_log!(
    "Not found {} {}",
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  Ok(TiiResponse::not_found_no_body())
}

pub(crate) fn default_not_acceptable_handler(
  request: &mut TiiRequestContext,
  _: &[TiiRouteable],
) -> TiiResult<TiiResponse> {
  info_log!(
    "Not Acceptable {} {}",
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  Ok(TiiResponse::not_acceptable_no_body())
}

pub(crate) fn default_method_not_allowed_handler(
  request: &mut TiiRequestContext,
  routes: &[TiiRouteable],
) -> TiiResult<TiiResponse> {
  info_log!(
    "Method not allowed {} {}",
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  let mut methods = HashSet::new();
  for route in routes {
    if matches!(route.matches(request), TiiRoutingDecision::MethodMismatch) {
      methods.insert(route.method().clone());
    }
  }

  let mut methods = methods.into_iter().collect::<Vec<_>>();
  methods.sort();

  Ok(TiiResponse::method_not_allowed(methods.as_slice()))
}

pub(crate) fn default_unsupported_media_type_handler(
  request: &mut TiiRequestContext,
  _: &[TiiRouteable],
) -> TiiResult<TiiResponse> {
  info_log!(
    "Unsupported Media Type {} {} {:?}",
    &request.request_head().get_method(),
    request.request_head().get_path(),
    request.request_head().get_content_type()
  );
  Ok(TiiResponse::unsupported_media_type_no_body())
}
