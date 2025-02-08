#[cfg(test)]
mod test {
  use tii::TiiStatusCode;

  #[test]
  fn test_from_code() {
    let valid_codes: [u16; 39] = [
      100, 101, 200, 201, 202, 203, 204, 205, 206, 300, 301, 302, 303, 304, 305, 307, 400, 401,
      403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 500, 501, 502,
      503, 504, 505,
    ];

    for code in valid_codes {
      assert!(TiiStatusCode::from_well_known_code(code).is_some(), "{}", code);
    }

    assert!(TiiStatusCode::from_well_known_code(69).is_none());
    assert!(TiiStatusCode::from_well_known_code(420).is_none());
    assert!(TiiStatusCode::from_well_known_code(1337).is_none());
  }

  #[test]
  fn test_into_code() {
    assert!(TiiStatusCode::from_well_known_code(200u16).is_some());
    assert!(TiiStatusCode::from_well_known_code(404u16).is_some());
    assert!(TiiStatusCode::from_well_known_code(1337u16).is_none());
  }

  #[test]
  fn test_into_string() {
    assert_eq!(TiiStatusCode::OK.status_line(), "OK");
    assert_eq!(TiiStatusCode::NotFound.status_line(), "Not Found");
    assert_eq!(TiiStatusCode::BadGateway.status_line(), "Bad Gateway");
  }
}
