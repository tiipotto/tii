use tii::HttpMethod;

#[test]
fn test_from_name() {
  assert_eq!(HttpMethod::from("GET"), HttpMethod::Get);
  assert_eq!(HttpMethod::from("POST"), HttpMethod::Post);
  assert_eq!(HttpMethod::from("PUT"), HttpMethod::Put);
  assert_eq!(HttpMethod::from("DELETE"), HttpMethod::Delete);
  assert_eq!(HttpMethod::from("Big"), HttpMethod::Custom("BIG".to_string()));
  assert_eq!(HttpMethod::from("sadNess"), HttpMethod::Custom("SADNESS".to_string()));
  assert_eq!(HttpMethod::from(""), HttpMethod::Custom("".to_string()));
}

#[test]
fn test_well_known() {
  for n in HttpMethod::well_known() {
    let n2 = HttpMethod::from(n.as_str());
    assert_eq!(n, &n2);
    assert!(n.is_well_known());
    assert!(!n.is_custom());
    assert!(n2.is_well_known());
    assert!(!n2.is_custom());
    assert_eq!(n.well_known_str().unwrap(), n.as_str());
    assert_eq!(n2.well_known_str().unwrap(), n.as_str());
    assert_eq!(n.as_str(), n.to_string().as_str());
    assert_eq!(n.as_str(), format!("{n2}").as_str());
  }
}

#[test]
fn test_well_custom() {
  let n = HttpMethod::from("sadNess");
  let n2 = HttpMethod::from(n.as_str());
  assert_eq!(n, n2);
  assert!(!n.is_well_known());
  assert!(n.is_custom());
  assert!(!n2.is_well_known());
  assert!(n2.is_custom());
  assert!(n.well_known_str().is_none());
  assert!(n2.well_known_str().is_none());
  assert_eq!("SADNESS", n.to_string().as_str());
  assert_eq!("SADNESS", format!("{n}").as_str());
}
