//! Contains all state that's needed to process a request.

use crate::http::headers::HttpHeaderName;
use crate::http::request::HttpVersion;
use crate::http::request_body::RequestBody;
use crate::http::RequestHead;
use crate::stream::ConnectionStream;
use crate::tii_error::{RequestHeadParsingError, TiiError, TiiResult};
use crate::tii_server::ConnectionStreamMetadata;
use crate::util::unwrap_some;
use crate::{debug_log, error_log, trace_log, util, warn_log, TypeSystem};
use std::any::Any;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::SystemTime;

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
      force_connection_close: false,
      stream_meta,
      routed_path: None,
      path_params: None,
      properties: None,
      type_system,
    }
  }

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
      force_connection_close: true,
      properties: None,
      routed_path: None,
      stream_meta,
      path_params: None,
      type_system,
    })
  }

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
      force_connection_close: true,
      properties: None,
      routed_path: None,
      stream_meta,
      path_params: None,
      type_system,
    })
  }

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

  /// Create a new RequestContext from a stream. This will parse RequestHead but not any part of the potencial request body.
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
