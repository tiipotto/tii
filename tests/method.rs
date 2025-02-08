use tii::TiiHttpMethod;

#[test]
fn test_from_name() {
  assert_eq!(TiiHttpMethod::from("GET"), TiiHttpMethod::Get);
  assert_eq!(TiiHttpMethod::from("POST"), TiiHttpMethod::Post);
  assert_eq!(TiiHttpMethod::from("PUT"), TiiHttpMethod::Put);
  assert_eq!(TiiHttpMethod::from("DELETE"), TiiHttpMethod::Delete);
  assert_eq!(TiiHttpMethod::from("Big"), TiiHttpMethod::Custom("BIG".to_string()));
  assert_eq!(TiiHttpMethod::from("sadNess"), TiiHttpMethod::Custom("SADNESS".to_string()));
  assert_eq!(TiiHttpMethod::from(""), TiiHttpMethod::Custom("".to_string()));
}

#[test]
fn test_well_known() {
  for n in TiiHttpMethod::well_known() {
    let n2 = TiiHttpMethod::from(n.as_str());
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
  let n = TiiHttpMethod::from("sadNess");
  let n2 = TiiHttpMethod::from(n.as_str());
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
