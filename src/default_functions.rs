use crate::http::request_context::RequestContext;
use crate::http::{Response, StatusCode};
use crate::{error_log, info_log};
use std::io;

pub(crate) fn default_pre_routing_filter(_request: &RequestContext) -> io::Result<bool> {
  Ok(true)
}

/// The default error handler for every Humpty app.
/// This can be overridden by using the `with_error_handler` method when building the app.
pub(crate) fn default_error_handler(
  request: &mut RequestContext,
  error: io::Error,
) -> io::Result<Response> {
  error_log!(
    "Internal Server Error {} {} {:?}",
    &request.request_head().method,
    request.request_head().path.as_str(),
    error
  );
  Ok(Response::empty(StatusCode::InternalServerError))
}

pub(crate) fn default_not_found_handler(request: &mut RequestContext) -> io::Result<Response> {
  info_log!(
    "Not found {} {}",
    &request.request_head().method,
    request.request_head().path.as_str()
  );
  Ok(Response::empty(StatusCode::NotFound))
}
