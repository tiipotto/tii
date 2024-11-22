//! Contains all state that's needed to process a request.

use crate::http::headers::HeaderName;
use crate::http::request::HttpVersion;
use crate::http::request_body::RequestBody;
use crate::http::RequestHead;
use crate::humpty_error::{HumptyError, HumptyResult, RequestHeadParsingError};
use crate::humpty_server::ConnectionStreamMetadata;
use crate::stream::ConnectionStream;
use crate::util;
use crate::util::unwrap_some;
use std::any::Any;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;

/// This struct contains all information needed to process a request as well as all state
/// for a single request.
#[derive(Debug)]
pub struct RequestContext {
  id: u128,
  address: String,
  request: RequestHead,
  body: Option<RequestBody>,
  force_connection_close: bool,
  stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,

  routed_path: Option<String>,

  path_params: Option<HashMap<String, String>>,

  ///TODO the key may be a candidate for Rc<str> instead of "String"?
  properties: Option<HashMap<String, Box<dyn Any + Send>>>,
}

impl RequestContext {
  /// Create a new RequestContext from a stream. This will parse RequestHead but not any part of the potencial request body.
  /// Errors on IO-Error or malformed RequestHead.
  pub fn new(
    stream: &dyn ConnectionStream,
    stream_meta: Option<Arc<dyn ConnectionStreamMetadata>>,
  ) -> HumptyResult<RequestContext> {
    let id = util::next_id();
    let address = stream.peer_addr()?;
    let req = RequestHead::new(stream)?;

    if req.version() == HttpVersion::Http09 {
      return Ok(RequestContext {
        id,
        address,
        request: req,
        body: None,
        force_connection_close: true,
        properties: None,
        routed_path: None,
        stream_meta,
        path_params: None,
      });
    }

    if req.version() == HttpVersion::Http11 {
      match req.get_header(&HeaderName::TransferEncoding) {
        Some("chunked") => {
          let body = RequestBody::new_chunked(stream.new_ref_read());
          return Ok(RequestContext {
            id,
            address,
            request: req,
            body: Some(body),
            force_connection_close: false,
            properties: None,
            routed_path: None,
            stream_meta,
            path_params: None,
          });
        }
        Some(other) => {
          return Err(HumptyError::from(RequestHeadParsingError::TransferEncodingNotSupported(
            other.to_string(),
          )))
        }
        None => {}
      }
    }

    if let Some(content_length) = req.get_header(&HeaderName::ContentLength) {
      let content_length: u64 = content_length.parse().map_err(|_| {
        HumptyError::from(RequestHeadParsingError::InvalidContentLength(content_length.to_string()))
      })?;

      let is_http_10 = req.version() == HttpVersion::Http10;

      if content_length == 0 {
        return Ok(RequestContext {
          id,
          address,
          request: req,
          body: None,
          force_connection_close: is_http_10,
          properties: None,
          routed_path: None,
          stream_meta,
          path_params: None,
        });
      }

      let body = RequestBody::new_with_content_length(stream.new_ref_read(), content_length);
      return Ok(RequestContext {
        id,
        address,
        request: req,
        body: Some(body),
        force_connection_close: is_http_10,
        properties: None,
        routed_path: None,
        stream_meta,
        path_params: None,
      });
    }

    Ok(RequestContext {
      id,
      address,
      request: req,
      body: None,
      force_connection_close: true,
      properties: None,
      routed_path: None,
      stream_meta,
      path_params: None,
    })
  }

  /// unique id for this request.
  pub fn id(&self) -> u128 {
    self.id
  }

  /// address of the peer we are talking to, entirely socket dependant.
  pub fn peer_address(&self) -> &str {
    self.address.as_str()
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
