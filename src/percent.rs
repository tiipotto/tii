//! Provides percent-encoding functionality.

/// A trait which represents the ability of a type to be percent-decoded.
pub trait PercentDecode {
  /// Attempt to percent-decode the value.
  fn percent_decode(&self) -> Option<Vec<u8>>;
}

impl<T> PercentDecode for T
where
  T: AsRef<str>,
{
  fn percent_decode(&self) -> Option<Vec<u8>> {
    let length = self.as_ref().len();
    let mut chars = self.as_ref().bytes();
    let mut decoded = Vec::with_capacity(length);

    while let Some(character) = chars.next() {
      if character == b'%' {
        let [hex_dig_1, hex_dig_2] = [chars.next()?, chars.next()?];
        let hex = format!("{}{}", hex_dig_1 as char, hex_dig_2 as char);
        let byte = u8::from_str_radix(&hex, 16).ok()?;
        decoded.push(byte);
      } else {
        decoded.push(character);
      }
    }

    Some(decoded)
  }
}

#[cfg(test)]
mod test {
  use crate::percent::PercentDecode;

  #[test]
  fn decode_unreserved_chars() {
    let string = "thisisatest";
    let decoded = string.percent_decode();

    assert_eq!(decoded, Some(string.as_bytes().to_vec()));
  }

  #[test]
  fn decode_reserved_chars() {
    let string = "this%20is%20a%20test%21%20%28and%20brackets%29";
    let decoded = string.percent_decode();

    assert_eq!(decoded, Some(b"this is a test! (and brackets)".to_vec()));
  }

  #[test]
  fn decode_bytes() {
    let string = "this%20is%20a%20%00null%20character";
    let decoded = string.percent_decode();

    assert_eq!(decoded, Some(b"this is a \0null character".to_vec()));
  }
}
