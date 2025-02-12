use tii::QValue;

#[test]
pub fn test() {
  for i in 0..=2000 {
    let qv = QValue::from_clamped(i);
    if i <= 1000 {
      assert_eq!(qv.as_u16(), i);
    }

    let str = qv.as_str();
    let display = qv.to_string();
    assert_eq!(str, display.as_str());
    let qv2 = QValue::parse(str).unwrap();
    assert_eq!(qv, qv2);
    let j = qv.as_str();
    match j.len() {
      3 => {
        assert_eq!(qv, QValue::parse(j.to_string() + "0").unwrap());
        assert_eq!(qv, QValue::parse(j.to_string() + "00").unwrap());
      }
      4 => {
        assert_eq!(qv, QValue::parse(j.to_string() + "0").unwrap());
      }
      5 => {}
      _ => panic!("{}", j),
    }
  }
}

#[test]
pub fn test_edge() {
  assert_eq!(1000, QValue::parse("1").unwrap().as_u16());
  assert_eq!(1000, QValue::parse("1.0").unwrap().as_u16());
  assert_eq!(1000, QValue::parse("1.00").unwrap().as_u16());
  assert_eq!(1000, QValue::parse("1.000").unwrap().as_u16());
  assert_eq!(0, QValue::parse("0").unwrap().as_u16());
  assert!(QValue::parse("2").is_none());
  assert!(QValue::parse("2.0").is_none());
  assert!(QValue::parse("1.001").is_none());
  assert!(QValue::parse("1.").is_none());
  assert!(QValue::parse("1.X").is_none());
  assert!(QValue::parse("0.X").is_none());
  assert!(QValue::parse("0.0X").is_none());
  assert!(QValue::parse("0.01X").is_none());
  assert!(QValue::parse("1.0X").is_none());
  assert!(QValue::parse("1X0").is_none());
  assert!(QValue::parse("1.0000").is_none());
}
