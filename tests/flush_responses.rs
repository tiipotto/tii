mod mock_stream;

use mock_stream::MockStream;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tii::{HttpVersion, IntoConnectionStream, MimeType, Response, ResponseBody, ResponseBodySink};

type BodyHandler = Box<dyn FnOnce(&dyn ResponseBodySink) -> std::io::Result<()> + Send>;

/// Writes two events, flushing after each (the second time through `as_write`).
/// Returns what reached the stream before the first write, after it, after each flush,
/// and after the response was written.
fn flush_snapshots(body: fn(BodyHandler) -> ResponseBody) -> [Vec<u8>; 5] {
  let stream = MockStream::without_data();
  let observer = stream.clone();
  let snapshots = Arc::new(Mutex::new(Vec::new()));
  let handler_snapshots = snapshots.clone();
  let response = Response::ok(
    body(Box::new(move |sink| {
      let snapshot =
        || handler_snapshots.lock().expect("poisoned").push(observer.copy_written_data());
      snapshot();
      sink.write_all(b"data: 1\n\n")?;
      snapshot();
      sink.flush()?;
      snapshot();
      sink.write_all(b"data: 2\n\n")?;
      sink.as_write().flush()?;
      snapshot();
      Ok(())
    })),
    MimeType::TextPlain,
  );
  let raw_stream = stream.clone().into_connection_stream();

  response.write_to(0, HttpVersion::Http11, raw_stream.as_stream_write()).expect("err");
  let mut snapshots = snapshots.lock().expect("poisoned").clone();
  snapshots.push(stream.copy_written_data());
  snapshots.try_into().expect("5 snapshots")
}

#[test]
fn test_chunked_response_flush() {
  let [before_write, after_write, after_flush, after_second_flush, done] =
    flush_snapshots(ResponseBody::chunked);
  let head = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\n\r\n";

  assert!(before_write.is_empty());
  assert!(after_write.is_empty());
  assert_eq!(String::from_utf8_lossy(&after_flush), format!("{head}9\r\ndata: 1\n\n\r\n"));
  assert_eq!(
    String::from_utf8_lossy(&after_second_flush),
    format!("{head}9\r\ndata: 1\n\n\r\n9\r\ndata: 2\n\n\r\n")
  );
  assert_eq!(
    String::from_utf8_lossy(&done),
    format!("{head}9\r\ndata: 1\n\n\r\n9\r\ndata: 2\n\n\r\n0\r\n\r\n")
  );
}

#[test]
fn test_streamed_response_flush() {
  let [before_write, after_write, after_flush, after_second_flush, done] =
    flush_snapshots(ResponseBody::streamed);
  let head = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n";

  assert!(before_write.is_empty());
  assert!(after_write.is_empty());
  assert_eq!(String::from_utf8_lossy(&after_flush), format!("{head}data: 1\n\n"));
  assert_eq!(String::from_utf8_lossy(&after_second_flush), format!("{head}data: 1\n\ndata: 2\n\n"));
  assert_eq!(after_second_flush, done);
}

/// Decodes the part of a chunked gzip response that has been written so far.
/// Returns the decoded data and whether the gzip stream was complete.
fn gunzip_chunked_body(raw: &[u8]) -> (Vec<u8>, bool) {
  let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("end of headers");
  let mut chunks = raw.split_at(header_end + 4).1;
  let mut gzip = Vec::new();
  while let Some(line_end) = chunks.windows(2).position(|w| w == b"\r\n") {
    let (size, rest) = chunks.split_at(line_end);
    let size = usize::from_str_radix(std::str::from_utf8(size).expect("utf8"), 16).expect("hex");
    let (chunk, rest) = rest.split_at(2 + size);
    gzip.extend_from_slice(chunk.split_at(2).1);
    chunks = rest.split_at(2).1;
  }

  let mut data = Vec::new();
  let complete = libflate::gzip::Decoder::new(gzip.as_slice())
    .expect("gzip header")
    .read_to_end(&mut data)
    .is_ok();
  (data, complete)
}

#[test]
fn test_chunked_gzip_response_flush() {
  let [before_write, after_write, after_flush, after_second_flush, done] =
    flush_snapshots(ResponseBody::chunked_gzip);

  assert!(before_write.is_empty());
  assert!(after_write.is_empty());
  assert_eq!(gunzip_chunked_body(&after_flush), (b"data: 1\n\n".to_vec(), false));
  assert_eq!(gunzip_chunked_body(&after_second_flush), (b"data: 1\n\ndata: 2\n\n".to_vec(), false));
  assert_eq!(gunzip_chunked_body(&done), (b"data: 1\n\ndata: 2\n\n".to_vec(), true));
}
