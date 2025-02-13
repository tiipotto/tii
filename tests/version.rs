use tii::HttpVersion;

#[test]
fn test_from_net_name() {
  assert_eq!(HttpVersion::try_from_net_str("HTTP/1.1"), Ok(HttpVersion::Http11));
  assert_eq!(HttpVersion::try_from_net_str("HTTP/1.0"), Ok(HttpVersion::Http10));
  assert_eq!(HttpVersion::try_from_net_str(""), Ok(HttpVersion::Http09));
  assert_eq!(HttpVersion::try_from_net_str("HTTP/420").unwrap_err(), "HTTP/420");
  assert_eq!(HttpVersion::try_from_net_str("HTTP/0.9").unwrap_err(), "HTTP/0.9");
}

#[test]
fn test_from_name() {
  assert_eq!(HttpVersion::try_from_str("HTTP/1.1"), Ok(HttpVersion::Http11));
  assert_eq!(HttpVersion::try_from_str("HTTP/1.0"), Ok(HttpVersion::Http10));
  assert_eq!(HttpVersion::try_from_str("").unwrap_err(), "");
  assert_eq!(HttpVersion::try_from_str("HTTP/420").unwrap_err(), "HTTP/420");
  assert_eq!(HttpVersion::try_from_str("HTTP/0.9"), Ok(HttpVersion::Http09));
}

#[test]
fn test_to_str() {
  assert_eq!(HttpVersion::Http11.as_str(), "HTTP/1.1");
  assert_eq!(HttpVersion::Http10.as_str(), "HTTP/1.0");
  assert_eq!(HttpVersion::Http09.as_str(), "HTTP/0.9");
}

#[test]
fn test_to_net_str() {
  assert_eq!(HttpVersion::Http11.as_net_str(), "HTTP/1.1");
  assert_eq!(HttpVersion::Http10.as_net_str(), "HTTP/1.0");
  assert_eq!(HttpVersion::Http09.as_net_str(), "");
}

#[test]
fn test_fmt() {
  assert_eq!(format!("{}", HttpVersion::Http11), "HTTP/1.1");
  assert_eq!(format!("{}", HttpVersion::Http10), "HTTP/1.0");
  assert_eq!(format!("{}", HttpVersion::Http09), "HTTP/0.9");
}
