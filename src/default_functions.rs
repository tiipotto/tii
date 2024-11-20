use crate::http::request_context::RequestContext;
use crate::http::{Response, StatusCode};
use crate::humpty_error::{HumptyError, HumptyResult};
use crate::humpty_router::{RouteHandler, RoutingDecision};
use crate::{error_log, info_log};
use std::collections::HashSet;

pub(crate) fn default_pre_routing_filter(_request: &RequestContext) -> HumptyResult<bool> {
  Ok(true)
}

/// The default error handler for every Humpty app.
/// This can be overridden by using the `with_error_handler` method when building the app.
pub(crate) fn default_error_handler(
  request: &mut RequestContext,
  error: HumptyError,
) -> HumptyResult<Response> {
  error_log!(
    "Internal Server Error {} {} {:?}",
    &request.request_head().method(),
    request.request_head().path(),
    error
  );
  Ok(Response::new(StatusCode::InternalServerError))
}

pub(crate) fn default_fallback_not_found_handler(
  request: &mut RequestContext,
) -> HumptyResult<Response> {
  info_log!(
    "Fallback: Not found {} {}",
    &request.request_head().method(),
    request.request_head().path()
  );
  Ok(Response::not_found_no_body())
}

pub(crate) fn default_not_found_handler(
  request: &mut RequestContext,
  _: &[RouteHandler],
) -> HumptyResult<Response> {
  info_log!("Not found {} {}", &request.request_head().method(), request.request_head().path());
  Ok(Response::not_found_no_body())
}

pub(crate) fn default_not_acceptable_handler(
  request: &mut RequestContext,
  _: &[RouteHandler],
) -> HumptyResult<Response> {
  info_log!(
    "Not Acceptable {} {}",
    &request.request_head().method(),
    request.request_head().path()
  );
  Ok(Response::not_acceptable_no_body())
}

pub(crate) fn default_method_not_allowed_handler(
  request: &mut RequestContext,
  routes: &[RouteHandler],
) -> HumptyResult<Response> {
  info_log!(
    "Method not allowed {} {}",
    &request.request_head().method(),
    request.request_head().path()
  );
  let mut methods = HashSet::new();
  for route in routes {
    if matches!(route.matches(request), RoutingDecision::MethodMismatch) {
      methods.insert(route.method.clone());
    }
  }

  let mut methods = methods.into_iter().collect::<Vec<_>>();
  methods.sort();

  Ok(Response::method_not_allowed(methods.as_slice()))
}

pub(crate) fn default_unsupported_media_type_handler(
  request: &mut RequestContext,
  _: &[RouteHandler],
) -> HumptyResult<Response> {
  info_log!(
    "Unsupported Media Type {} {} {:?}",
    &request.request_head().method(),
    request.request_head().path(),
    request.request_head().get_content_type()
  );
  Ok(Response::unsupported_media_type_no_body())
}
