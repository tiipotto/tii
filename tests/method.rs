use tii::http::method::Method;

#[test]
fn test_from_name() {
  assert_eq!(Method::from("GET"), Method::Get);
  assert_eq!(Method::from("POST"), Method::Post);
  assert_eq!(Method::from("PUT"), Method::Put);
  assert_eq!(Method::from("DELETE"), Method::Delete);
  assert_eq!(Method::from("Big"), Method::Custom("BIG".to_string()));
  assert_eq!(Method::from("sadNess"), Method::Custom("SADNESS".to_string()));
  assert_eq!(Method::from(""), Method::Custom("".to_string()));
}

#[test]
fn test_well_known() {
  for n in Method::well_known() {
    let n2 = Method::from(n.as_str());
    assert_eq!(n, &n2);
    assert!(n.is_well_known());
    assert!(!n.is_custom());
    assert!(n2.is_well_known());
    assert!(!n2.is_custom());
    assert_eq!(n.well_known_str().unwrap(), n.as_str());
    assert_eq!(n2.well_known_str().unwrap(), n.as_str());
    assert_eq!(n.as_str(), n.to_string().as_str());
    assert_eq!(n.as_str(), format!("{}", n2).as_str());
  }
}

#[test]
fn test_well_custom() {
  let n = Method::from("sadNess");
  let n2 = Method::from(n.as_str());
  assert_eq!(n, n2);
  assert!(!n.is_well_known());
  assert!(n.is_custom());
  assert!(!n2.is_well_known());
  assert!(n2.is_custom());
  assert!(n.well_known_str().is_none());
  assert!(n2.well_known_str().is_none());
  assert_eq!("SADNESS", n.to_string().as_str());
  assert_eq!("SADNESS", format!("{}", n).as_str());
}
