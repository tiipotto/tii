//! Contains all state that's needed to process a request.

use crate::http::headers::HttpHeaderName;
use crate::http::request::HttpVersion;
use crate::http::request_body::RequestBody;
use crate::http::RequestHead;
use crate::stream::ConnectionStream;
use crate::tii_error::{RequestHeadParsingError, TiiError, TiiResult};
use crate::tii_server::ConnectionStreamMetadata;
use crate::util::unwrap_some;
use crate::{
  debug_log, error_log, trace_log, util, warn_log, TypeSystem, TypeSystemError,
  UserError,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::io::ErrorKind;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use std::{io, mem};

/// This struct contains all information needed to process a request as well as all state
/// for a single request.
#[derive(Debug)]
pub struct RequestContext {
  id: u128,
  timestamp: u128,
  peer_address: String,
  local_address: String,
  request: RequestHead,
  body: Option<RequestBody>,
  request_entity: Option<Box<dyn Any + Send + Sync>>,
  force_connection_close: bool,
  stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
  routed_path: Option<String>,
  path_params: Option<HashMap<String, String>>,
  properties: Option<HashMap<String, Box<dyn Any + Send>>>,
  type_system: TypeSystem,
}

impl RequestContext {
  /// Create a new RequestContext programmatically.
  /// This is useful for unit testing endpoints.
  pub fn new(
    id: u128,
    peer_address: impl ToString,
    local_address: impl ToString,
    head: RequestHead,
    body: Option<RequestBody>,
    stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
    type_system: TypeSystem,
  ) -> Self {
    Self {
      id,
      timestamp: SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|a| a.as_millis())
        .unwrap_or(0),
      peer_address: peer_address.to_string(),
      local_address: local_address.to_string(),
      request: head,
      body,
      request_entity: None,
      force_connection_close: false,
      stream_meta,
      routed_path: None,
      path_params: None,
      properties: None,
      type_system,
    }
  }

  #[allow(clippy::too_many_arguments)]
  fn new_http09(
    id: u128,
    timestamp: u128,
    local_address: String,
    peer_address: String,
    req: RequestHead,
    _stream: &dyn ConnectionStream,
    stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
    type_system: TypeSystem,
  ) -> TiiResult<RequestContext> {
    trace_log!("tii: Request {id} is http 0.9");

    Ok(RequestContext {
      id,
      timestamp,
      peer_address,
      local_address,
      request: req,
      body: None,
      request_entity: None,
      force_connection_close: true,
      properties: None,
      routed_path: None,
      stream_meta,
      path_params: None,
      type_system,
    })
  }

  #[allow(clippy::too_many_arguments)]
  fn new_http10(
    id: u128,
    timestamp: u128,
    local_address: String,
    peer_address: String,
    req: RequestHead,
    stream: &dyn ConnectionStream,
    stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
    type_system: TypeSystem,
  ) -> TiiResult<RequestContext> {
    trace_log!("tii: Request {id} is http 1.0");

    if let Some(content_length) = req.get_header(&HttpHeaderName::ContentLength) {
      let content_length: u64 = content_length.parse().map_err(|_| {
        TiiError::from(RequestHeadParsingError::InvalidContentLength(content_length.to_string()))
      })?;

      if content_length == 0 {
        trace_log!("tii: Request {id} has no request body");
        return Ok(RequestContext {
          id,
          timestamp,
          peer_address,
          local_address,
          request: req,
          body: None,
          request_entity: None,
          force_connection_close: true,
          properties: None,
          routed_path: None,
          stream_meta,
          path_params: None,
          type_system,
        });
      }

      trace_log!("tii: Request {id} has {content_length} bytes of request body");
      let body = RequestBody::new_with_content_length(stream.new_ref_read(), content_length);
      return Ok(RequestContext {
        id,
        timestamp,
        peer_address,
        local_address,
        request: req,
        body: Some(body),
        request_entity: None,
        force_connection_close: true,
        properties: None,
        routed_path: None,
        stream_meta,
        path_params: None,
        type_system,
      });
    }

    trace_log!(
      "tii: Request {id} did not sent Content-Length header. Assuming that it has no request body"
    );
    Ok(RequestContext {
      id,
      timestamp,
      peer_address,
      local_address,
      request: req,
      body: None,
      request_entity: None,
      force_connection_close: true,
      properties: None,
      routed_path: None,
      stream_meta,
      path_params: None,
      type_system,
    })
  }

  #[allow(clippy::too_many_arguments)]
  fn new_http11(
    id: u128,
    timestamp: u128,
    local_address: String,
    peer_address: String,
    req: RequestHead,
    stream: &dyn ConnectionStream,
    stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
    type_system: TypeSystem,
  ) -> TiiResult<RequestContext> {
    trace_log!("tii: Request {id} is http 1.1");

    let content_length =
      if let Some(content_length) = req.get_header(&HttpHeaderName::ContentLength) {
        Some(content_length.parse::<u64>().map_err(|_| {
          TiiError::from(RequestHeadParsingError::InvalidContentLength(content_length.to_string()))
        })?)
      } else {
        None
      };

    match (
      req.get_header(&HttpHeaderName::ContentEncoding),
      req.get_header(&HttpHeaderName::TransferEncoding),
    ) {
      (None, None) => match content_length {
        None => {
          if req.get_header(&HttpHeaderName::Connection) != Some("keep-alive") {
            trace_log!(
              "tii: Request {id} did not sent Content-Length header. Assuming that it has no request body. Connection: keep-alive was not explicitly requested, so will send Connection: close");

            return Ok(RequestContext {
              id,
              timestamp,
              peer_address,
              local_address,
              request: req,
              body: None,
              request_entity: None,
              force_connection_close: true,
              properties: None,
              routed_path: None,
              stream_meta,
              path_params: None,
              type_system,
            });
          }

          if req.get_method().is_likely_to_have_request_body() {
            warn_log!(
            "tii: Request {id} did not sent Content-Length header but did request Connection: keep-alive. Assuming that it has no request body. The request method {} usually has a body, will force Connection: close to be safe.", req.get_method()
            );

            return Ok(RequestContext {
              id,
              timestamp,
              peer_address,
              local_address,
              request: req,
              body: None,
              request_entity: None,
              force_connection_close: true,
              properties: None,
              routed_path: None,
              stream_meta,
              path_params: None,
              type_system,
            });
          }

          trace_log!(
            "tii: Request {id} did not sent Content-Length header. Assuming that it has no request body. Connection: keep-alive was requested, so will trust the client that the request actually has no body.");

          Ok(RequestContext {
            id,
            timestamp,
            peer_address,
            local_address,
            request: req,
            body: None,
            request_entity: None,
            force_connection_close: false,
            properties: None,
            routed_path: None,
            stream_meta,
            path_params: None,
            type_system,
          })
        }
        Some(0) => {
          trace_log!("tii: Request {id} has no request body");
          Ok(RequestContext {
            id,
            timestamp,
            peer_address,
            local_address,
            request: req,
            body: None,
            request_entity: None,
            force_connection_close: false,
            properties: None,
            routed_path: None,
            stream_meta,
            path_params: None,
            type_system,
          })
        }
        Some(content_length) => {
          trace_log!("tii: Request {id} has {content_length} bytes of request body");
          Ok(RequestContext {
            id,
            timestamp,
            peer_address,
            local_address,
            request: req,
            body: Some(RequestBody::new_with_content_length(stream.new_ref_read(), content_length)),
            request_entity: None,
            force_connection_close: false,
            properties: None,
            routed_path: None,
            stream_meta,
            path_params: None,
            type_system,
          })
        }
      },
      (None, Some("chunked")) => {
        trace_log!("tii: Request {id} has chunked request body");
        let body = RequestBody::new_chunked(stream.new_ref_read());
        Ok(RequestContext {
          id,
          timestamp,
          peer_address,
          local_address,
          request: req,
          body: Some(body),
          request_entity: None,
          force_connection_close: false,
          properties: None,
          routed_path: None,
          stream_meta,
          path_params: None,
          type_system,
        })
      }
      (None, Some("x-gzip")) | (None, Some("gzip")) => {
        trace_log!("tii: Request {id} has gzip request body with length of uncompressed content");
        let Some(content_length) = content_length else {
          error_log!("tii: Request {id} not implemented no transfer encoding, Content-Encoding: gzip/x-gzip without Content-Length header");
          return Err(TiiError::from(RequestHeadParsingError::ContentLengthHeaderMissing));
        };

        let body =
          RequestBody::new_gzip_with_uncompressed_length(stream.new_ref_read(), content_length)?;

        Ok(RequestContext {
          id,
          timestamp,
          peer_address,
          local_address,
          request: req,
          body: Some(body),
          //TODO, i have seen gzip produce trailer bytes in the past that are just padding
          //and I am not confident enough that libflate consumes them.
          //Until I have verified that libflate consumes the trailerbytes without fail we should not enable keep alive.
          request_entity: None,
          force_connection_close: true,
          properties: None,
          routed_path: None,
          stream_meta,
          path_params: None,
          type_system,
        })
      }
      (Some("gzip"), None) | (Some("x-gzip"), None) => {
        trace_log!("tii: Request {id} has gzip request body with length of compressed content");
        //gzip+Content-Length of zipped stuff
        let Some(content_length) = content_length else {
          error_log!("tii: Request {id} not implemented Transfer-Encoding: gzip/x-gzip, no Content-Encoding without Content-Length header");
          return Err(TiiError::from(RequestHeadParsingError::ContentLengthHeaderMissing));
        };

        //TODO curl, hyper and several http server implementation disagree on how this should be handled.
        //Its safe to assume that no client will ever send this...
        //We may have to read the full rfc eventually, the rfc only mentions that this exists and
        //This impl is honestly based upon some forum comments of a obscure http proxy.
        let body = RequestBody::new_gzip_with_compressed_content_length(
          stream.new_ref_read(),
          content_length,
        )?;
        Ok(RequestContext {
          id,
          timestamp,
          peer_address,
          local_address,
          request: req,
          body: Some(body),
          request_entity: None,
          force_connection_close: false,
          properties: None,
          routed_path: None,
          stream_meta,
          path_params: None,
          type_system,
        })
      }
      (Some("gzip"), Some("chunked"))
      | (Some("x-gzip"), Some("chunked"))
      | (None, Some("gzip, chunked"))
      | (None, Some("x-gzip, chunked")) => {
        trace_log!("tii: Request {id} has chunked gzip request body");
        let body = RequestBody::new_gzip_chunked(stream.new_ref_read())?;
        Ok(RequestContext {
          id,
          timestamp,
          peer_address,
          local_address,
          request: req,
          body: Some(body),
          request_entity: None,
          force_connection_close: false,
          properties: None,
          routed_path: None,
          stream_meta,
          path_params: None,
          type_system,
        })
      }
      (other_encoding, other_transfer) => {
        match other_transfer {
          Some("chunked") | None => (),
          Some(other) => {
            error_log!("tii: Request {id} has unimplemented transfer encoding: {}", other);
            return Err(TiiError::from(RequestHeadParsingError::TransferEncodingNotSupported(
              other.to_string(),
            )));
          }
        }

        let Some(other_encoding) = other_encoding else {
          error_log!(
            "tii: BUG! Fatal unreachable syntax/encoding reached {:?} {:?}",
            other_encoding,
            other_transfer
          );
          util::unreachable();
        };

        error_log!("tii: Request {id} has unimplemented content encoding: {}", other_encoding);

        Err(TiiError::from(RequestHeadParsingError::ContentEncodingNotSupported(
          other_encoding.to_string(),
        )))
      }
    }
  }

  /// Create a new RequestContext from a stream. This will parse RequestHead but not any part of the potential request body.
  /// Errors on IO-Error or malformed RequestHead.
  pub fn read(
    stream: &dyn ConnectionStream,
    stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
    max_head_buffer_size: usize,
    type_system: TypeSystem,
  ) -> TiiResult<RequestContext> {
    let now: u128 =
      SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|a| a.as_millis()).unwrap_or(0);
    let id = util::next_id();
    let peer_address = stream.peer_addr()?;
    let local_address = stream.local_addr()?;
    debug_log!("tii: Request {id} local: {} peer: {}", &local_address, &peer_address);

    let req = RequestHead::read(id, stream, max_head_buffer_size)?;

    match req.get_version() {
      HttpVersion::Http09 => Self::new_http09(
        id,
        now,
        local_address,
        peer_address,
        req,
        stream,
        stream_meta,
        type_system,
      ),
      HttpVersion::Http10 => Self::new_http10(
        id,
        now,
        local_address,
        peer_address,
        req,
        stream,
        stream_meta,
        type_system,
      ),
      HttpVersion::Http11 => Self::new_http11(
        id,
        now,
        local_address,
        peer_address,
        req,
        stream,
        stream_meta,
        type_system,
      ),
    }
  }

  /// unique id for this request.
  pub fn id(&self) -> u128 {
    self.id
  }

  /// returns the timestamp when this request began parsing from the stream.
  /// This timestamp is in unix epoch millis.
  /// Meaning milliseconds passed since Midnight 1. Jan 1970 in UTC timezone (UK/England).
  /// The time source of this timestamp is not monotonic.
  pub fn get_timestamp(&self) -> u128 {
    self.timestamp
  }

  /// address of the peer we are talking to, entirely socket dependant.
  pub fn peer_address(&self) -> &str {
    self.peer_address.as_str()
  }

  /// address of our socket
  pub fn local_address(&self) -> &str {
    self.local_address.as_str()
  }

  /// True if the request contains the specified property.
  pub fn contains_property<K: AsRef<str>>(&self, key: K) -> bool {
    if let Some(prop) = self.properties.as_ref() {
      return prop.contains_key(key.as_ref());
    }
    false
  }

  /// Get the specified property, uses downcast ref. returns none if the downcast didn't succeed.
  pub fn get_property<T: Any + Send, K: AsRef<str>>(&self, key: K) -> Option<&T> {
    if let Some(prop) = self.properties.as_ref() {
      if let Some(value) = prop.get(key.as_ref()) {
        return value.downcast_ref::<T>();
      }
    }

    None
  }

  /// Gets a downcast to the stream metadata. returns none if the downcast didn't succeed or there is no meta.
  pub fn get_stream_meta<T: ConnectionStreamMetadata>(&self) -> Option<&T> {
    if let Some(arc) = self.stream_meta.as_ref() {
      return arc.as_ref().as_any().downcast_ref::<T>();
    }

    None
  }

  /// Removes a property from the request.
  pub fn remove_property<K: AsRef<str>>(&mut self, key: K) -> Option<Box<dyn Any + Send>> {
    if let Some(prop) = self.properties.as_mut() {
      if let Some(value) = prop.remove(key.as_ref()) {
        return Some(value);
      }
    }
    None
  }

  /// Sets a property into the request.
  pub fn set_property<T: Any + Send, K: ToString>(
    &mut self,
    key: K,
    value: T,
  ) -> Option<Box<dyn Any + Send>> {
    let boxed = Box::new(value) as Box<dyn Any + Send>;

    let k = key.to_string();
    if let Some(prop) = self.properties.as_mut() {
      if let Some(value) = prop.insert(k, boxed) {
        return Some(value);
      }
      return None;
    }

    //Lazy init the map.
    let mut nmap = HashMap::new();
    nmap.insert(k, boxed);
    self.properties = Some(nmap);
    None
  }

  /// Returns an iterator over property keys.
  pub fn get_property_keys(&self) -> Box<dyn Iterator<Item = &String> + '_> {
    match self.properties.as_ref() {
      Some(props) => Box::new(props.iter().map(|(k, _)| k)),
      None => Box::new(std::iter::empty()),
    }
  }

  /// Returns the parsed request entity if any
  /// Parsing (if required by Endpoint) happens after the routing decision has been made.
  /// Before that this fn will always return None
  pub fn get_request_entity(&self) -> Option<&(dyn Any + Send + Sync)> {
    self.request_entity.as_ref().map(Box::as_ref)
  }

  /// Returns the mutable parsed request entity if any
  /// Parsing (if required by Endpoint) happens after the routing decision has been made.
  /// Before that this fn will always return None
  pub fn get_request_entity_mut(&mut self) -> Option<&mut (dyn Any + Send + Sync)> {
    self.request_entity.as_mut().map(Box::as_mut)
  }

  /// Replaces the request entity.
  /// Beware that setting the request entity to an incorrect type the endpoint does not expect will cause
  /// UserError before the endpoint is invoked!
  pub fn set_request_entity(
    &mut self,
    entity: Option<Box<dyn Any + Send + Sync>>,
  ) -> Option<Box<dyn Any + Send + Sync>> {
    mem::replace(&mut self.request_entity, entity)
  }

  /// Calls a closure with the cast request entity.
  /// # Errors
  /// If casting fails because the request entity is not of a compatible type
  pub fn cast_request_entity<DST: Any + ?Sized + 'static, RET: Any + 'static>(
    &self,
    receiver: impl FnOnce(&DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    let src = self.get_request_entity().ok_or(TypeSystemError::SourceTypeUnknown)?;

    let caster = self.type_system.type_cast_wrapper(src.type_id(), TypeId::of::<DST>())?;

    caster.call(src, receiver)
  }

  /// Calls a closure with the cast mutable request entity.
  /// # Errors
  /// If casting fails because the request entity is not of a compatible type
  pub fn cast_request_entity_mut<DST: Any + ?Sized + 'static, RET: Any + 'static>(
    &mut self,
    receiver: impl FnOnce(&mut DST) -> RET + 'static,
  ) -> Result<RET, TypeSystemError> {
    let src =
      self.request_entity.as_mut().map(Box::as_mut).ok_or(TypeSystemError::SourceTypeUnknown)?;

    let caster = self.type_system.type_cast_wrapper_mut(Any::type_id(src), TypeId::of::<DST>())?;

    caster.call(src, receiver)
  }

  /// Ref to request head.
  pub fn request_head(&self) -> &RequestHead {
    &self.request
  }

  /// Ref to mutable request head.
  pub fn request_head_mut(&mut self) -> &mut RequestHead {
    &mut self.request
  }

  /// Ref to body.
  pub fn request_body(&self) -> Option<&RequestBody> {
    self.body.as_ref()
  }

  /// Get the routed path, yields "" before routing.
  pub fn routed_path(&self) -> &str {
    self.routed_path.as_deref().unwrap_or("")
  }

  /// get the path param keys.
  pub fn get_path_param_keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
    match self.path_params.as_ref() {
      Some(props) => Box::new(props.iter().map(|(k, _)| k.as_str())),
      None => Box::new(std::iter::empty()),
    }
  }

  /// get that path param key value pairs
  pub fn get_path_params(&self) -> Box<dyn Iterator<Item = (&str, &str)> + '_> {
    match self.path_params.as_ref() {
      Some(props) => Box::new(props.iter().map(|(k, v)| (k.as_str(), v.as_str()))),
      None => Box::new(std::iter::empty()),
    }
  }

  /// Gets a path param or None
  pub fn get_path_param(&self, asr: impl AsRef<str>) -> Option<&str> {
    if let Some(path) = self.path_params.as_ref() {
      return path.get(asr.as_ref()).map(|e| e.as_str());
    }

    None
  }

  ///
  /// This fn will parse a path param using its FromStr trait.
  ///
  /// # Errors
  /// - TiiError::UserError(UserError::InvalidPathParameter)
  ///   If the FromStr function fails. For example, you try to parse a number, and it's not a number.
  ///   This is reasonably common and your error handler should generally map this into a http 400 bad request response of some sorts.
  ///
  /// - TiiError::UserError(UserError::MissingPathParameter)
  ///   If the path parameter does not exist for the endpoint.
  ///   This is usually indicative of an error in the program, because an endpoint should not request path parameter by name that don't exist for its path.
  ///   Most likely scenario for this to happen, is if the path parameter was renamed in the path constant of the endpoint,
  ///   but not inside the actual endpoint handler.
  ///   The error handle should either abort the program or return http 500.
  ///
  /// # Example
  /// ```rust
  ///use std::num::ParseIntError;
  ///use tii::{MimeType, RequestContext, Response, TiiError, TiiResult, UserError};
  ///
  /// //Your endpoint at for example path '/some/path/{id}/whatever'
  /// pub fn endpoint(ctx: &RequestContext) -> TiiResult<Response> {
  ///  let id : u128 = ctx.parse_path_param("id")?;
  ///  Ok(Response::ok(format!("The id is {id}"), MimeType::TextPlain))
  /// }
  ///
  /// //Global error handler
  ///pub fn error_handler(ctx: &mut RequestContext, error: TiiError) -> TiiResult<Response> {
  ///  if let TiiError::UserError(UserError::InvalidPathParameter(name, _type, error)) = &error {
  ///     return Ok(Response::bad_request(format!("The path parameter {name} is invalid. error={error}"), MimeType::TextPlain));
  ///  }
  ///
  ///  if let TiiError::UserError(UserError::MissingPathParameter(name)) = &error {
  ///    eprintln!("The endpoint {} is buggy or mis-routed, its requesting path param {name}, but no such path param exist.", ctx.routed_path());
  ///    return Ok(Response::internal_server_error_no_body());
  ///  }
  ///
  ///  //Handle other errors here
  ///  todo!()
  ///}
  ///
  /// ```
  ///
  pub fn parse_path_param<T: Any + FromStr<Err = E>, E: Error + Send + Sync + 'static>(
    &self,
    name: impl AsRef<str>,
  ) -> TiiResult<T> {
    let name = name.as_ref();
    self
      .path_params
      .as_ref()
      .and_then(|params| params.get(name))
      .ok_or(TiiError::UserError(UserError::MissingPathParameter(name.to_string())))?
      .parse::<T>()
      .map_err(|e| {
        TiiError::UserError(UserError::InvalidPathParameter(
          name.to_string(),
          TypeId::of::<T>(),
          Box::new(e),
        ))
      })
  }

  /// Sets a path param.
  pub fn set_path_param(&mut self, key: impl ToString, value: impl ToString) -> Option<String> {
    if let Some(path) = self.path_params.as_mut() {
      return path.insert(key.to_string(), value.to_string());
    }

    self.path_params = Some(HashMap::new());
    unwrap_some(self.path_params.as_mut()).insert(key.to_string(), value.to_string());

    None
  }

  /// Sets the routed path, this is called after routing is performed.
  /// Calling this in a pre routing filter has no effect on routing.
  /// Calling this in a post routing filter will overwrite the value the endpoint sees.
  pub fn set_routed_path<T: ToString>(&mut self, rp: T) {
    self.routed_path.replace(rp.to_string());
  }

  /// Replaces the request body with a new one (or none).
  /// The old body if any is consumed/discarded.
  pub fn set_body_consume_old(&mut self, body: Option<RequestBody>) -> io::Result<()> {
    if let Some(old_body) = self.body.as_ref() {
      consume_body(old_body)?
    }
    self.body = body;
    Ok(())
  }

  /// Forces the Connection to be closed after the request is handled.
  /// This is sensible if errors are encountered.
  pub fn force_connection_close(&mut self) {
    self.force_connection_close = true;
  }

  /// Returns true if the connection will forcibly be closed after the request is handled.
  pub fn is_connection_close_forced(&self) -> bool {
    self.force_connection_close
  }

  /// Fully consumes the current request body.
  /// The body itself will remain valid, just yield EOF as soon as read.
  /// Calling this multiple times is a noop.
  pub fn consume_request_body(&self) -> io::Result<()> {
    if let Some(body) = self.body.as_ref() {
      consume_body(body)?
    }
    Ok(())
  }

  pub(crate) fn get_type_system(&self) -> &TypeSystem {
    &self.type_system
  }
}

/// utility ot consume the body.
fn consume_body(body: &RequestBody) -> io::Result<()> {
  let mut discarding_buffer = [0; 0x1_00_00]; //TODO heap alloc maybe? cfg-if!
  loop {
    let discarded = body.read(discarding_buffer.as_mut_slice()).or_else(|e| {
      if e.kind() == ErrorKind::UnexpectedEof {
        Ok(0)
      } else {
        Err(e)
      }
    })?; //Not so unexpected eof!

    if discarded == 0 {
      return Ok(());
    }
  }
}
