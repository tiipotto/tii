use crate::RequestContext;
use crate::{Response, StatusCode};
use crate::{Routeable, RoutingDecision};
use crate::{TiiError, TiiResult};
use crate::{error_log, info_log};
use std::collections::HashSet;

pub(crate) fn default_pre_routing_filter(_request: &RequestContext) -> TiiResult<bool> {
  Ok(true)
}

/// The default error handler for every Tii app.
/// This can be overridden by using the `with_error_handler` method when building the app.
pub(crate) fn default_error_handler(
  request: &mut RequestContext,
  error: TiiError,
) -> TiiResult<Response> {
  error_log!(
    "Request {} Internal Server Error {} {} {:?}",
    request.id(),
    &request.request_head().get_method(),
    request.request_head().get_path(),
    error
  );
  Ok(Response::new(StatusCode::InternalServerError))
}

pub(crate) fn default_fallback_not_found_handler(
  request: &mut RequestContext,
) -> TiiResult<Response> {
  info_log!(
    "Request {} Fallback: Not found {} {}",
    request.id(),
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  Ok(Response::not_found_no_body())
}

pub(crate) fn default_not_found_handler(
  request: &mut RequestContext,
  _: &[Routeable],
) -> TiiResult<Response> {
  info_log!(
    "Request {} Not found {} {}",
    request.id(),
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  Ok(Response::not_found_no_body())
}

pub(crate) fn default_not_acceptable_handler(
  request: &mut RequestContext,
  _: &[Routeable],
) -> TiiResult<Response> {
  info_log!(
    "Request {} Not Acceptable {} {}",
    request.id(),
    &request.request_head().get_method(),
    request.request_head().get_path()
  );
  Ok(Response::not_acceptable_no_body())
}

pub(crate) fn default_method_not_allowed_handler(
  request: &mut RequestContext,
  routes: &[Routeable],
) -> TiiResult<Response> {
  info_log!(
    "Request {} Method not allowed {} {}",
    request.id(),
    request.request_head().get_method(),
    request.request_head().get_path()
  );
  let mut methods = HashSet::new();
  for route in routes {
    if matches!(route.matches(request), RoutingDecision::MethodMismatch) {
      methods.insert(route.get_method().clone());
    }
  }

  let mut methods = methods.into_iter().collect::<Vec<_>>();
  methods.sort();

  Ok(Response::method_not_allowed(methods.as_slice()))
}

pub(crate) fn default_unsupported_media_type_handler(
  request: &mut RequestContext,
  _: &[Routeable],
) -> TiiResult<Response> {
  info_log!(
    "Request {} Unsupported Media Type {} {} {:?}",
    request.id(),
    request.request_head().get_method(),
    request.request_head().get_path(),
    request.request_head().get_content_type()
  );
  Ok(Response::unsupported_media_type_no_body())
}
