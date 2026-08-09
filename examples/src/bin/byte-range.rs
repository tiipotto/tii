//! Minimal server repro for benchmarking HTTP Range support.
//!
//! Run with: cargo run --bin byte-range -- /path/to/video.mp4
//!
//! Benchmark:
//!   curl -o /dev/null -v -w "TTFB: %{time_starttransfer}s  Total: %{time_total}s  Size: %{size_download} bytes\n" http://127.0.0.1:8080/video.mp4
//!   curl -o /dev/null -v -H "Range: bytes=0-1023" -w "TTFB: %{time_starttransfer}s  Total: %{time_total}s  Size: %{size_download} bytes\n" http://127.0.0.1:8080/video.mp4

use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;

use log::LevelFilter;
use tii::extras::{Connector, TcpConnector};
use tii::{
  HttpHeaderName, HttpMethod, MimeType, RequestContext, Response, ResponseBody, ServerBuilder,
  StatusCode, TiiResult,
};

const ADDR: &str = "0.0.0.0:8080";

fn maybe_omit_body_for_head(ctx: &RequestContext, mut response: Response) -> Response {
  if ctx.get_method() == HttpMethod::Head {
    response.omit_body = true;
  }
  response
}

fn serve_bytes(ctx: &RequestContext) -> Response {
  eprintln!("[byte-range] {} {} headers:", ctx.get_method(), ctx.get_path(),);
  for h in ctx.iter_headers() {
    eprintln!("[byte-range]   {}: {}", h.name.to_str(), h.value);
  }

  let file_path = std::env::args().nth(1).unwrap_or_else(|| "test.mp4".to_string());
  let path = PathBuf::from(&file_path);

  let mime = MimeType::from_extension(path.extension().and_then(|e| e.to_str()).unwrap_or(""));

  let mut file = match File::open(&path) {
    Ok(f) => f,
    Err(e) => {
      eprintln!("[byte-range] failed to open {}: {e}", file_path);
      return Response::not_found("not found", MimeType::TextPlain);
    }
  };

  file.seek(SeekFrom::End(0)).expect("seek to end");
  let file_size = file.stream_position().expect("stream_position");
  file.seek(SeekFrom::Start(0)).expect("seek to start");

  let range_header = if ctx.get_method() != HttpMethod::Get {
    None
  } else {
    ctx.get_header(HttpHeaderName::Range).and_then(|header| {
      let (unit, _) = header.trim().split_once('=')?;
      unit.eq_ignore_ascii_case("bytes").then_some(header)
    })
  };

  let Some(range_header) = range_header else {
    eprintln!("[byte-range] serving {} ({file_size} bytes) as {mime:?} (full file)", file_path,);
    let body = ResponseBody::from_file(file).unwrap_or_else(|e| {
      eprintln!("[byte-range] error creating body: {e}");
      std::process::exit(1);
    });
    let mut response = Response::ok(body, mime);
    response.add_header(HttpHeaderName::AcceptRanges, "bytes").unwrap();
    return maybe_omit_body_for_head(ctx, response);
  };

  let Some(byte_range) = ctx.get_byte_range() else {
    eprintln!("[byte-range] invalid Range header: {range_header}");
    return Response::bad_request("invalid range", MimeType::TextPlain);
  };

  let Some((offset, end_inclusive, length)) = byte_range.get_values_for_size(file_size) else {
    eprintln!("[byte-range] unsatisfiable range: {range_header} (file_size={file_size})");
    let response = Response::new(StatusCode::RequestedRangeNotSatisfiable)
      .with_header(HttpHeaderName::ContentRange, format!("bytes */{file_size}"))
      .unwrap();
    return maybe_omit_body_for_head(ctx, response);
  };

  eprintln!(
    "[byte-range] serving {} range {}-{}/{} ({} bytes) as {mime:?}",
    file_path, offset, end_inclusive, file_size, length,
  );

  let body = ResponseBody::from_file_ranged(file, offset, length).unwrap_or_else(|e| {
    eprintln!("[byte-range] error creating range body: {e}");
    std::process::exit(1);
  });
  let content_range = byte_range.get_content_range_header_for_size(file_size).unwrap();
  let response = Response::partial_content(body, mime)
    .with_header(HttpHeaderName::ContentRange, content_range)
    .unwrap()
    .with_header(HttpHeaderName::AcceptRanges, "bytes")
    .unwrap();
  maybe_omit_body_for_head(ctx, response)
}

fn main() -> TiiResult<()> {
  trivial_log::init_std(LevelFilter::Trace).unwrap();

  let tii_server = ServerBuilder::builder_arc(|builder| {
    builder.router(|router| {
      router.route_get("/video.mp4", serve_bytes)?.route_head("/video.mp4", serve_bytes)
    })
  })?;

  eprintln!("[byte-range] listening on {ADDR}");
  eprintln!("[byte-range] usage:");
  eprintln!("[byte-range]   Full file:");
  eprintln!(
    "[byte-range]     curl -o /dev/null -v -w 'TTFB: %{{time_starttransfer}}s  Total: %{{time_total}}s  Size: %{{size_download}} bytes\\n' http://{ADDR}/video.mp4"
  );
  eprintln!("[byte-range]   Range request:");
  eprintln!(
    "[byte-range]     curl -o /dev/null -v -H 'Range: bytes=0-1023' -w 'TTFB: %{{time_starttransfer}}s  Total: %{{time_total}}s  Size: %{{size_download}} bytes\\n' http://{ADDR}/video.mp4"
  );

  let _ = TcpConnector::start_unpooled(ADDR, tii_server)?.join(None);
  Ok(())
}
