use crate::http::request_context::RequestContext;
use crate::http::{Response, StatusCode};
use crate::humpty_error::{HumptyError, HumptyResult};
use crate::{error_log, info_log};

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

pub(crate) fn default_not_found_handler(request: &mut RequestContext) -> HumptyResult<Response> {
  info_log!("Not found {} {}", &request.request_head().method(), request.request_head().path());
  Ok(Response::new(StatusCode::NotFound))
}
