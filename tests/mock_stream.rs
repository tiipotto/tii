#![allow(dead_code)]
use tii::stream::{ConnectionStream, IntoConnectionStream};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct MockStream {
  read_data: Arc<Mutex<VecDeque<u8>>>,
  write_data: Arc<Mutex<Vec<u8>>>,
}

impl MockStream {
  pub fn with_str(data: &str) -> Self {
    Self::with_data(VecDeque::from_iter(data.to_string().bytes()))
  }

  pub fn with_slice(data: &[u8]) -> Self {
    Self::with_data(VecDeque::from(data.to_vec()))
  }

  pub fn with_data(data: VecDeque<u8>) -> Self {
    Self { read_data: Arc::new(Mutex::new(data)), write_data: Arc::new(Mutex::new(Vec::new())) }
  }

  pub fn without_data() -> Self {
    Self::with_data(VecDeque::new())
  }

  pub fn copy_written_data(&self) -> Vec<u8> {
    self.write_data.lock().unwrap().clone()
  }

  pub fn copy_written_data_to_string(&self) -> String {
    String::from_utf8_lossy(self.copy_written_data().as_slice()).to_string()
  }

  pub fn to_stream(&self) -> Box<dyn ConnectionStream> {
    self.clone().into_connection_stream()
  }
}

impl IntoConnectionStream for MockStream {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream> {
    let cl = self.clone();
    (Box::new(cl) as Box<dyn Read + Send>, Box::new(self) as Box<dyn Write + Send>)
      .into_connection_stream()
  }
}

impl Write for MockStream {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.write_data.lock().unwrap().write(buf)
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

impl Read for MockStream {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut bytes_written: usize = 0;

    for byte in buf {
      if let Some(new_byte) = self.read_data.lock().unwrap().pop_front() {
        *byte = new_byte;
        bytes_written += 1;
      } else {
        return Ok(bytes_written);
      }
    }

    Ok(bytes_written)
  }
}
