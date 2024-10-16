const ALPHABET: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A trait which represents the ability to be base64 encoded.
/// This trait is implemented for all types which can be converted to a slice of bytes.
pub trait Base64Encode {
  /// Encode self into a base64 string.
  fn encode(&self) -> String;
}

impl<T> Base64Encode for T
where
  T: AsRef<[u8]>,
{
  fn encode(&self) -> String {
    let bytes = self.as_ref();
    let mut result = String::with_capacity((bytes.len() * 4) / 3 + 4);

    for group_index in 0..bytes.len() / 3 {
      let group = &bytes[group_index * 3..group_index * 3 + 3];

      result.push(ALPHABET[(group[0] >> 2) as usize] as char);
      result.push(ALPHABET[((group[0] & 0x03) << 4 | group[1] >> 4) as usize] as char);
      result.push(ALPHABET[((group[1] & 0x0f) << 2 | group[2] >> 6) as usize] as char);
      result.push(ALPHABET[group[2] as usize & 0x3f] as char);
    }

    let remaining = bytes.len() % 3;
    let group = &bytes[(bytes.len() - remaining)..];
    if remaining == 1 {
      result.push(ALPHABET[(group[0] >> 2) as usize] as char);
      result.push(ALPHABET[((group[0] & 0x03) << 4) as usize] as char);
      result.push('=');
      result.push('=');
    } else if remaining == 2 {
      result.push(ALPHABET[(group[0] >> 2) as usize] as char);
      result.push(ALPHABET[((group[0] & 0x03) << 4 | group[1] >> 4) as usize] as char);
      result.push(ALPHABET[((group[1] & 0x0f) << 2) as usize] as char);
      result.push('=');
    }

    result
  }
}

#[test]
fn test_base64_encode() {
  let padding_0_input = "foo";
  let padding_0_expected = "Zm9v";
  let padding_0_result = padding_0_input.encode();
  assert_eq!(padding_0_result, padding_0_expected);

  let padding_1_input = "yeet";
  let padding_1_expected = "eWVldA==";
  let padding_1_result = padding_1_input.encode();
  assert_eq!(padding_1_result, padding_1_expected);

  let padding_2_input = "hello";
  let padding_2_expected = "aGVsbG8=";
  let padding_2_result = padding_2_input.encode();
  assert_eq!(padding_2_result, padding_2_expected);
}
