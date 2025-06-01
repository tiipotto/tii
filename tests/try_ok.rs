use std::fs;
use std::fs::File;
use std::path::PathBuf;
use tii::{MimeType, Response};

#[test]
#[cfg(unix)]
pub fn test_try_ok() {
  {
    let file = File::create("/tmp/try_ok").unwrap();
    Response::try_ok(file, MimeType::ApplicationOctetStream).unwrap();
  }
  fs::remove_file(PathBuf::from("/tmp/try_ok")).unwrap();
}
