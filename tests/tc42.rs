use std::fmt::Debug;
use std::io::{Read, Write};
use std::time::Duration;
use tii::RequestContext;
use tii::ServerBuilder;
use tii::TiiResult;
use tii::{
  ConnectionStream, ConnectionStreamRead, ConnectionStreamWrite, IntoConnectionStream, Response,
};

mod mock_stream;

fn dummy_route(_: &RequestContext) -> TiiResult<Response> {
  unreachable!()
}

/// This stream is simulation of poor impl that says it has something to read but when actually read is called it reads nothing.
/// We shouldn't write anything to such a stream so all write fns panic.
#[derive(Debug)]
struct BadStream;

impl IntoConnectionStream for BadStream {
  fn into_connection_stream(self) -> Box<dyn ConnectionStream> {
    Box::new(BadStream)
  }
}

impl ConnectionStreamRead for BadStream {
  fn read(&self, _buf: &mut [u8]) -> std::io::Result<usize> {
    Ok(0)
  }

  fn ensure_readable(&self) -> std::io::Result<bool> {
    Ok(true)
  }

  fn available(&self) -> usize {
    1
  }

  fn read_until(&self, _end: u8, _limit: usize, _buf: &mut Vec<u8>) -> std::io::Result<usize> {
    Ok(0)
  }

  fn read_exact(&self, _buf: &mut [u8]) -> std::io::Result<()> {
    Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
  }

  fn new_ref_read(&self) -> Box<dyn Read + Send + Sync> {
    Box::new(BadStream)
  }

  fn as_stream_read(&self) -> &dyn ConnectionStreamRead {
    self
  }

  fn new_ref_stream_read(&self) -> Box<dyn ConnectionStreamRead> {
    Box::new(BadStream)
  }

  fn set_read_timeout(&self, _dur: Option<Duration>) -> std::io::Result<()> {
    Ok(())
  }

  fn get_read_timeout(&self) -> std::io::Result<Option<Duration>> {
    Ok(None)
  }
}

impl Read for BadStream {
  fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
    Ok(0)
  }
}

impl ConnectionStreamWrite for BadStream {
  fn write(&self, _buf: &[u8]) -> std::io::Result<usize> {
    panic!("Should not be called")
  }

  fn write_all(&self, _buf: &[u8]) -> std::io::Result<()> {
    panic!("Should not be called")
  }

  fn flush(&self) -> std::io::Result<()> {
    panic!("Should not be called")
  }

  fn set_write_timeout(&self, _dur: Option<Duration>) -> std::io::Result<()> {
    Ok(())
  }

  fn get_write_timeout(&self) -> std::io::Result<Option<Duration>> {
    Ok(None)
  }

  fn new_ref_write(&self) -> Box<dyn Write + Send + Sync> {
    Box::new(BadStream)
  }

  fn new_ref_stream_write(&self) -> Box<dyn ConnectionStreamWrite> {
    Box::new(BadStream)
  }

  fn as_stream_write(&self) -> &dyn ConnectionStreamWrite {
    self
  }
}

impl Write for BadStream {
  fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
    panic!("Should not be called")
  }

  fn flush(&mut self) -> std::io::Result<()> {
    panic!("Should not be called")
  }
}

impl ConnectionStream for BadStream {
  fn new_ref(&self) -> Box<dyn ConnectionStream> {
    Box::new(BadStream)
  }

  fn peer_addr(&self) -> std::io::Result<String> {
    Ok("127.0.0.1".to_owned())
  }

  fn local_addr(&self) -> std::io::Result<String> {
    Ok("127.0.0.1".to_owned())
  }
}

#[test]
pub fn tc42() {
  let server =
    ServerBuilder::default().router(|rt| rt.route_any("/*", dummy_route)).expect("ERR").build();

  let stream = BadStream;
  let err = server.handle_connection(stream).unwrap_err();
  assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}
