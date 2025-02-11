use tii::http::headers::HeaderName;

#[test]
fn test_well_known_header_types() {
  for n in HeaderName::well_known() {
    assert!(n.is_well_known());
    assert!(!n.is_custom());
    assert_eq!(n.to_str(), n.well_known_str().unwrap());
    let hdr = HeaderName::from(n.to_str());
    assert!(hdr.is_well_known(), "{}", n);
    assert!(!hdr.is_custom());
    assert_eq!(n, &hdr);
  }
}

#[test]
fn test_custom_header() {
  let hdr = HeaderName::from("X-Custom");
  assert!(!hdr.is_well_known());
  assert!(hdr.is_custom());
  let hdr2 = HeaderName::from(hdr.to_str());
  assert!(!hdr2.is_well_known());
  assert!(hdr2.is_custom());
  assert_eq!(&hdr2, &hdr);
}

#[test]
fn test_header_sort_by_name() {
  let x = HeaderName::well_known();
  let mut v = x.to_vec();
  v.push(HeaderName::from("Baba-Yaga"));
  v.push(HeaderName::from("Abc-Man"));
  v.sort();

  let mut v2: Vec<String> = x.iter().map(|x| x.to_str().to_string()).collect();
  v2.push(String::from("Abc-Man"));
  v2.push(String::from("Baba-Yaga"));
  v2.sort();

  for x in 0..v.len() {
    assert_eq!(v[x].to_string(), v2[x]);
  }
}
