use tii::TiiHttpVersion;

#[test]
fn test_from_net_name() {
  assert_eq!(TiiHttpVersion::try_from_net_str("HTTP/1.1"), Ok(TiiHttpVersion::Http11));
  assert_eq!(TiiHttpVersion::try_from_net_str("HTTP/1.0"), Ok(TiiHttpVersion::Http10));
  assert_eq!(TiiHttpVersion::try_from_net_str(""), Ok(TiiHttpVersion::Http09));
  assert_eq!(TiiHttpVersion::try_from_net_str("HTTP/420").unwrap_err(), "HTTP/420");
  assert_eq!(TiiHttpVersion::try_from_net_str("HTTP/0.9").unwrap_err(), "HTTP/0.9");
}

#[test]
fn test_from_name() {
  assert_eq!(TiiHttpVersion::try_from_str("HTTP/1.1"), Ok(TiiHttpVersion::Http11));
  assert_eq!(TiiHttpVersion::try_from_str("HTTP/1.0"), Ok(TiiHttpVersion::Http10));
  assert_eq!(TiiHttpVersion::try_from_str("").unwrap_err(), "");
  assert_eq!(TiiHttpVersion::try_from_str("HTTP/420").unwrap_err(), "HTTP/420");
  assert_eq!(TiiHttpVersion::try_from_str("HTTP/0.9"), Ok(TiiHttpVersion::Http09));
}

#[test]
fn test_to_str() {
  assert_eq!(TiiHttpVersion::Http11.as_str(), "HTTP/1.1");
  assert_eq!(TiiHttpVersion::Http10.as_str(), "HTTP/1.0");
  assert_eq!(TiiHttpVersion::Http09.as_str(), "HTTP/0.9");
}

#[test]
fn test_to_net_str() {
  assert_eq!(TiiHttpVersion::Http11.as_net_str(), "HTTP/1.1");
  assert_eq!(TiiHttpVersion::Http10.as_net_str(), "HTTP/1.0");
  assert_eq!(TiiHttpVersion::Http09.as_net_str(), "");
}

#[test]
fn test_fmt() {
  assert_eq!(format!("{}", TiiHttpVersion::Http11), "HTTP/1.1");
  assert_eq!(format!("{}", TiiHttpVersion::Http10), "HTTP/1.0");
  assert_eq!(format!("{}", TiiHttpVersion::Http09), "HTTP/0.9");
}
