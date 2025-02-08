use tii::{TiiAcceptMimeType, TiiAcceptQualityMimeType, TiiMimeGroup, TiiMimeType, TiiQValue};

#[test]
fn test_not_valid() {
  assert!(TiiMimeType::parse("*/*").is_none());
  assert!(TiiMimeType::parse("image/*").is_none());
  assert!(TiiMimeType::parse("image/meep/mop").is_none());
  assert!(TiiMimeType::parse("image/meep/").is_none());
  assert!(TiiMimeType::parse("/jpeg").is_none());
  assert!(TiiMimeType::parse("a\0a/bla").is_none());
  assert!(TiiMimeType::parse("image/asäb").is_none());
  assert!(TiiMimeType::parse("image/JPEG").is_none());
  assert!(TiiMimeType::parse("fubar").is_none());
}
#[test]
fn test_valid() {
  assert!(TiiMimeType::parse("image/meep+mop").is_some());
  assert!(TiiMimeType::parse("application/json+dicom").is_some());
}

#[test]
fn test_well_known() {
  for n in TiiMimeType::well_known() {
    let n2 = TiiMimeType::parse(n.as_str()).unwrap();
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
      let from_ext = TiiMimeType::from_extension(ext);
      assert_eq!(&from_ext, n);
    }
  }
}

#[test]
fn test_custom() {
  let n = TiiMimeType::parse("application/sadness").unwrap();
  let n2 = TiiMimeType::parse(n.as_str()).unwrap();
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
  let special = TiiMimeType::from_extension("superspecial");
  assert_eq!(special, TiiMimeType::ApplicationOctetStream);
}

#[test]
fn test_well_known_groups() {
  for n in TiiMimeGroup::well_known() {
    let n2 = TiiMimeGroup::parse(n.as_str()).unwrap();
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
  let n = TiiMimeGroup::parse("special").unwrap();
  let n2 = TiiMimeGroup::parse(n.as_str()).unwrap();
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
  assert_eq!(TiiAcceptMimeType::Wildcard, TiiAcceptMimeType::parse("*/*").unwrap());
  assert_eq!(
    TiiAcceptMimeType::GroupWildcard(TiiMimeGroup::Video),
    TiiAcceptMimeType::parse("video/*").unwrap()
  );
  assert_eq!(
    TiiAcceptMimeType::GroupWildcard(TiiMimeGroup::Audio),
    TiiAcceptMimeType::parse("audio/*").unwrap()
  );
  assert_eq!(
    TiiAcceptMimeType::Specific(TiiMimeType::ApplicationJson),
    TiiAcceptMimeType::parse("application/json").unwrap()
  );
  assert!(TiiAcceptMimeType::parse("no*/fun*").is_none());
  assert!(TiiAcceptMimeType::parse("application/fun*").is_none());
  assert!(TiiAcceptMimeType::parse("application/").is_none());
  assert!(TiiAcceptMimeType::parse("application").is_none());
}

#[test]
fn test_acceptable_permits_group() {
  assert!(TiiAcceptMimeType::from(TiiMimeGroup::Video).permits_group(TiiMimeGroup::Video));
  assert!(TiiAcceptMimeType::from(TiiMimeGroup::Video).permits_group(&TiiMimeGroup::Video));
  assert!(!TiiAcceptMimeType::from(TiiMimeGroup::Audio).permits_group(TiiMimeGroup::Video));
  assert!(TiiAcceptMimeType::Wildcard.permits_group(TiiMimeGroup::Video));
  assert!(TiiAcceptMimeType::Wildcard.permits_group(TiiMimeGroup::Audio));
  assert!(
    !TiiAcceptMimeType::from(TiiMimeType::ApplicationJson).permits_group(TiiMimeGroup::Application)
  );
  assert!(!TiiAcceptMimeType::from(TiiMimeType::ApplicationJson).permits_group(TiiMimeGroup::Video));
  assert!(TiiAcceptMimeType::from(TiiMimeGroup::parse("fubar").unwrap())
    .permits_group(TiiMimeGroup::parse("fubar").unwrap()));
  assert!(!TiiAcceptMimeType::from(TiiMimeGroup::parse("fubar").unwrap())
    .permits_group(TiiMimeGroup::Video));
}

#[test]
fn test_acceptable_permits() {
  assert!(TiiAcceptMimeType::from(TiiMimeGroup::Video)
    .permits(TiiAcceptMimeType::from(TiiMimeType::VideoMp4)));
  assert!(TiiAcceptMimeType::from(TiiMimeGroup::Video)
    .permits(TiiAcceptMimeType::from(TiiMimeGroup::Video)));
  assert!(TiiAcceptMimeType::from(TiiMimeGroup::Video)
    .permits(TiiAcceptMimeType::from(TiiMimeGroup::Video)));
  assert!(!TiiAcceptMimeType::from(TiiMimeGroup::Video).permits(TiiAcceptMimeType::Wildcard));
  assert!(TiiAcceptMimeType::Wildcard.permits(TiiAcceptMimeType::Wildcard));
}

#[test]
fn test_acceptable_display_and_parse() {
  for n in TiiMimeGroup::well_known() {
    let orig = TiiAcceptMimeType::from(n);
    let parsed = TiiAcceptMimeType::parse(orig.to_string()).unwrap();
    assert_eq!(orig, parsed);
  }

  for n in TiiMimeType::well_known() {
    let orig = TiiAcceptMimeType::from(n);
    let parsed = TiiAcceptMimeType::parse(orig.to_string()).unwrap();
    assert_eq!(orig, parsed);
  }

  assert_eq!(
    TiiAcceptMimeType::parse(TiiAcceptMimeType::Wildcard.to_string()).unwrap(),
    TiiAcceptMimeType::Wildcard
  )
}

#[test]
fn test_accept_q_display_and_parse() {
  for n in TiiMimeGroup::well_known() {
    for i in 0..1000 {
      let q = TiiQValue::from_clamped(i);
      let orig = TiiAcceptQualityMimeType::from_group(n.clone(), q);
      let parsed =
        TiiAcceptQualityMimeType::parse(orig.to_string()).unwrap().into_iter().next().unwrap();

      assert_eq!(orig, parsed);
      assert_eq!(n, parsed.group().unwrap());
      assert!(parsed.mime().is_none());
      assert!(!parsed.is_wildcard());
      assert!(parsed.is_group_wildcard());
      assert!(!parsed.is_specific());
    }
  }

  for n in TiiMimeType::well_known() {
    for i in 0..1000 {
      let q = TiiQValue::from_clamped(i);
      let orig = TiiAcceptQualityMimeType::from_mime(n.clone(), q);
      let parsed =
        TiiAcceptQualityMimeType::parse(orig.to_string()).unwrap().into_iter().next().unwrap();
      assert_eq!(orig, parsed);
      assert_eq!(n.mime_group(), parsed.group().unwrap());
      assert_eq!(n, parsed.mime().unwrap());
      assert!(!parsed.is_wildcard());
      assert!(!parsed.is_group_wildcard());
      assert!(parsed.is_specific());

      let accq = TiiAcceptMimeType::from(parsed);
      match accq {
        TiiAcceptMimeType::Specific(t) => {
          assert_eq!(t, n.clone());
        }
        _ => panic!("{}", accq.to_string()),
      }
    }
  }

  for i in 0..1000 {
    let q = TiiQValue::from_clamped(i);
    let orig = TiiAcceptQualityMimeType::wildcard(q);
    let parsed =
      TiiAcceptQualityMimeType::parse(orig.to_string()).unwrap().into_iter().next().unwrap();
    assert_eq!(orig, parsed);
    assert!(parsed.group().is_none());
    assert!(parsed.is_wildcard());
    assert!(!parsed.is_group_wildcard());
    assert!(!parsed.is_specific());
  }

  assert_eq!(
    TiiAcceptQualityMimeType::default(),
    TiiAcceptQualityMimeType::wildcard(TiiQValue::default())
  );
}

#[test]
fn test_accept_q_edge() {
  assert_eq!(
    TiiAcceptQualityMimeType::parse("application/json;q=0.500")
      .unwrap()
      .into_iter()
      .next()
      .unwrap(),
    TiiAcceptQualityMimeType::from_mime(TiiMimeType::ApplicationJson, TiiQValue::from_clamped(500))
  );
  assert!(TiiAcceptQualityMimeType::parse("application/json;sad=0.500").is_none());
  assert!(TiiAcceptQualityMimeType::parse("application/json;q=4.0").is_none());
  assert!(TiiAcceptQualityMimeType::parse("application/*j;q=1.0").is_none());
  assert!(TiiAcceptQualityMimeType::parse("app*/json;q=1.0").is_none());
  assert!(TiiAcceptQualityMimeType::parse("application/*j").is_none());
  assert!(TiiAcceptQualityMimeType::parse("app*/json").is_none());
  assert_eq!(
    TiiAcceptQualityMimeType::parse("application/*").unwrap().into_iter().next().unwrap(),
    TiiAcceptQualityMimeType::from_group(TiiMimeGroup::Application, TiiQValue::from_clamped(1000))
  );
}

#[test]
fn test_accept_q_parse_all() {
  let mut types: Vec<TiiAcceptQualityMimeType> = Vec::new();
  for n in TiiMimeType::well_known() {
    types.push(TiiAcceptQualityMimeType::from_mime(n.clone(), TiiQValue::from_clamped(500)));
  }

  let hdr_value = TiiAcceptQualityMimeType::elements_to_header_value(&types);
  let parsed_types = TiiAcceptQualityMimeType::parse(hdr_value).unwrap();
  assert_eq!(types, parsed_types);
}

#[test]
fn test_mime_type2group() {
  for n in TiiMimeType::well_known() {
    let x = TiiMimeGroup::from(n);
    assert_eq!(x, TiiMimeGroup::parse(n.as_str()).unwrap(), "{}", n);
  }

  assert_eq!(
    TiiMimeGroup::from(TiiMimeType::parse("application/dubdub").unwrap()),
    TiiMimeGroup::parse("application/dubdub").unwrap()
  );
}
