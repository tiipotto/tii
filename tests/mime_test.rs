use humpty::http::mime::{AcceptMimeType, AcceptQualityMimeType, MimeGroup, MimeType, QValue};

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

#[test]
fn test_well_known_groups() {
  for n in MimeGroup::well_known() {
    let n2 = MimeGroup::parse(n.as_str()).unwrap();
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
fn test_custom_group() {
  let n = MimeGroup::parse("special").unwrap();
  let n2 = MimeGroup::parse(n.as_str()).unwrap();
  assert_eq!(n, n2);
  assert!(!n.is_well_known());
  assert!(n.is_custom());
  assert!(!n2.is_well_known());
  assert!(n2.is_custom());
  assert!(n.well_known_str().is_none());
  assert!(n2.well_known_str().is_none());
  assert_eq!("special", n.to_string().as_str());
  assert_eq!("special", format!("{}", n).as_str());
}

#[test]
fn test_acceptable() {
  assert_eq!(AcceptMimeType::Wildcard, AcceptMimeType::parse("*/*").unwrap());
  assert_eq!(
    AcceptMimeType::GroupWildcard(MimeGroup::Video),
    AcceptMimeType::parse("video/*").unwrap()
  );
  assert_eq!(
    AcceptMimeType::GroupWildcard(MimeGroup::Audio),
    AcceptMimeType::parse("audio/*").unwrap()
  );
  assert_eq!(
    AcceptMimeType::Specific(MimeType::ApplicationJson),
    AcceptMimeType::parse("application/json").unwrap()
  );
  assert!(AcceptMimeType::parse("no*/fun*").is_none());
  assert!(AcceptMimeType::parse("application/fun*").is_none());
  assert!(AcceptMimeType::parse("application/").is_none());
  assert!(AcceptMimeType::parse("application").is_none());
}

#[test]
fn test_acceptable_permits_group() {
  assert!(AcceptMimeType::from(MimeGroup::Video).permits_group(MimeGroup::Video));
  assert!(AcceptMimeType::from(MimeGroup::Video).permits_group(&MimeGroup::Video));
  assert!(!AcceptMimeType::from(MimeGroup::Audio).permits_group(MimeGroup::Video));
  assert!(AcceptMimeType::Wildcard.permits_group(MimeGroup::Video));
  assert!(AcceptMimeType::Wildcard.permits_group(MimeGroup::Audio));
  assert!(!AcceptMimeType::from(MimeType::ApplicationJson).permits_group(MimeGroup::Application));
  assert!(!AcceptMimeType::from(MimeType::ApplicationJson).permits_group(MimeGroup::Video));
  assert!(AcceptMimeType::from(MimeGroup::parse("fubar").unwrap())
    .permits_group(MimeGroup::parse("fubar").unwrap()));
  assert!(!AcceptMimeType::from(MimeGroup::parse("fubar").unwrap()).permits_group(MimeGroup::Video));
}

#[test]
fn test_acceptable_permits() {
  assert!(AcceptMimeType::from(MimeGroup::Video).permits(AcceptMimeType::from(MimeType::VideoMp4)));
  assert!(AcceptMimeType::from(MimeGroup::Video).permits(AcceptMimeType::from(MimeGroup::Video)));
  assert!(AcceptMimeType::from(MimeGroup::Video).permits(&AcceptMimeType::from(MimeGroup::Video)));
  assert!(!AcceptMimeType::from(MimeGroup::Video).permits(AcceptMimeType::Wildcard));
  assert!(AcceptMimeType::Wildcard.permits(AcceptMimeType::Wildcard));
}

#[test]
fn test_acceptable_display_and_parse() {
  for n in MimeGroup::well_known() {
    let orig = AcceptMimeType::from(n);
    let parsed = AcceptMimeType::parse(orig.to_string()).unwrap();
    assert_eq!(orig, parsed);
  }

  for n in MimeType::well_known() {
    let orig = AcceptMimeType::from(n);
    let parsed = AcceptMimeType::parse(orig.to_string()).unwrap();
    assert_eq!(orig, parsed);
  }

  assert_eq!(
    AcceptMimeType::parse(AcceptMimeType::Wildcard.to_string()).unwrap(),
    AcceptMimeType::Wildcard
  )
}

#[test]
fn test_accept_q_display_and_parse() {
  for n in MimeGroup::well_known() {
    for i in 0..1000 {
      let q = QValue::from_clamped(i);
      let orig = AcceptQualityMimeType::from_group(n.clone(), q);
      let parsed =
        AcceptQualityMimeType::parse(orig.to_string()).unwrap().into_iter().next().unwrap();
      assert_eq!(orig, parsed);
    }
  }

  for n in MimeType::well_known() {
    for i in 0..1000 {
      let q = QValue::from_clamped(i);
      let orig = AcceptQualityMimeType::from_mime(n.clone(), q);
      let parsed =
        AcceptQualityMimeType::parse(orig.to_string()).unwrap().into_iter().next().unwrap();
      assert_eq!(orig, parsed);
    }
  }

  for i in 0..1000 {
    let q = QValue::from_clamped(i);
    let orig = AcceptQualityMimeType::wildcard(q);
    let parsed =
      AcceptQualityMimeType::parse(orig.to_string()).unwrap().into_iter().next().unwrap();
    assert_eq!(orig, parsed);
  }
}
