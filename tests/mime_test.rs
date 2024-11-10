use humpty::http::mime::MimeType;

#[test]
fn test_not_valid() {
  assert!(MimeType::parse("*/*").is_none());
  assert!(MimeType::parse("image/*").is_none());
  assert!(MimeType::parse("image/meep/mop").is_none());
  assert!(MimeType::parse("image/meep/").is_none());
  assert!(MimeType::parse("/jpeg").is_none());
  assert!(MimeType::parse("a\0a/bla").is_none());
  assert!(MimeType::parse("image/asäb").is_none());
  assert!(MimeType::parse("image/JPEG").is_none());
  assert!(MimeType::parse("fubar").is_none());
}
#[test]
fn test_valid() {
  assert!(MimeType::parse("image/meep+mop").is_some());
  assert!(MimeType::parse("application/json+dicom").is_some());
}

#[test]
fn test_well_known() {
  for n in MimeType::well_known() {
    let n2 = MimeType::parse(n.as_str()).unwrap();
    assert_eq!(n, &n2);
    assert!(n.is_well_known());
    assert!(!n.is_custom());
    assert!(n2.is_well_known());
    assert!(!n2.is_custom());
    assert_eq!(n.well_known_str().unwrap(), n.as_str());
    assert_eq!(n2.well_known_str().unwrap(), n.as_str());
    assert_eq!(n.as_str(), n.to_string().as_str());
    assert_eq!(n.as_str(), format!("{}", n2).as_str());
    assert!(!n.extension().is_empty());
    assert!(n.extension().is_ascii());

    if n.has_unique_known_extension() {
      let ext = n.extension();
      let from_ext = MimeType::from_extension(ext);
      assert_eq!(&from_ext, n);
    }
  }
}

#[test]
fn test_custom() {
  let n = MimeType::parse("application/sadness").unwrap();
  let n2 = MimeType::parse(n.as_str()).unwrap();
  assert_eq!(n, n2);
  assert!(!n.is_well_known());
  assert!(n.is_custom());
  assert!(!n2.is_well_known());
  assert!(n2.is_custom());
  assert!(n.well_known_str().is_none());
  assert!(n2.well_known_str().is_none());
  assert_eq!("application/sadness", n.to_string().as_str());
  assert_eq!("application/sadness", format!("{}", n).as_str());
  assert!(!n.has_unique_known_extension());
  assert!(!n.extension().is_empty());
  assert!(n.extension().is_ascii());
}

#[test]
fn test_custom_extension() {
  let special = MimeType::from_extension("superspecial");
  assert_eq!(special, MimeType::ApplicationOctetStream);
}
