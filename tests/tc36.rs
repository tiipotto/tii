use crate::mock_stream::MockStream;
use tii::RequestContext;
use tii::Response;
use tii::ServerBuilder;
use tii::TiiResult;

mod mock_stream;

fn filter(ctx: &mut RequestContext) -> TiiResult<()> {
  ctx.request_head_mut().add_header("test", "testo")?;
  ctx.request_head_mut().add_header("test", "testo2")?;

  ctx.request_head_mut().add_query_param("beep", "beep1");
  ctx.request_head_mut().add_query_param("beep", "beep2");

  let r = ctx.request_head_mut().set_query_param("mog", "cog");
  assert_eq!(r.len(), 2);
  assert_eq!("bog", r.first().unwrap().to_string());
  assert_eq!("log", r.get(1).unwrap().to_string());

  ctx.request_head_mut().set_query_param("zog", "hog");

  let r = ctx.request_head_mut().remove_query_params("rm");
  assert_eq!(r.len(), 2);
  assert_eq!("1", r.first().unwrap().to_string());
  assert_eq!("2", r.get(1).unwrap().to_string());

  let x = ctx.request_head().get_query();
  assert_eq!(("bla".to_string(), "xxxx".to_string()), x[0]);
  assert_eq!(("bla".to_string(), "yyyyy".to_string()), x[1]);
  assert_eq!(("zog".to_string(), "hog".to_string()), x[2]);
  assert_eq!(("beep".to_string(), "beep1".to_string()), x[3]);
  assert_eq!(("beep".to_string(), "beep2".to_string()), x[4]);
  assert_eq!(("mog".to_string(), "cog".to_string()), x[5]);

  Ok(())
}

fn route(ctx: &RequestContext) -> Response {
  assert_eq!(Some("testo"), ctx.request_head().get_header("test"));
  assert_eq!(Some("xxxx"), ctx.request_head().get_query_param("bla"));

  let p = ctx.request_head().get_headers("test");
  assert_eq!(p.len(), 2);
  assert_eq!("testo", p.first().unwrap().to_string());
  assert_eq!("testo2", p.get(1).unwrap().to_string());

  let p = ctx.request_head().get_query_params("bla");
  assert_eq!(p.len(), 2);
  assert_eq!("xxxx", p.first().unwrap().to_string());
  assert_eq!("yyyyy", p.get(1).unwrap().to_string());

  let p = ctx.request_head().get_query_params("beep");
  assert_eq!(p.len(), 2);
  assert_eq!("beep1", p.first().unwrap().to_string());
  assert_eq!("beep2", p.get(1).unwrap().to_string());

  let p = ctx.request_head().get_query_params("mog");
  assert_eq!(p.len(), 1);
  assert_eq!("cog", p.first().unwrap().to_string());

  assert_eq!(ctx.request_head().get_query_param("nope"), None);
  assert_eq!(ctx.request_head().get_query_param("rm"), None);
  assert_eq!(ctx.request_head().get_query_params("rm").len(), 0);

  Response::no_content()
}

#[test]
pub fn tc36() {
  let server = ServerBuilder::default()
    .router(|rt| rt.with_request_filter(filter)?.route_any("/dummy", route))
    .expect("ERR")
    .build();

  let stream = MockStream::with_str(
    "GET /dummy?bla=xxxx&bla=yyyyy&mog=bog&mog=log&zog=fog&zog=hog&rm=1&rm=2 HTTP/1.1\r\nHdr: test\r\n\r\n",
  );
  let con = stream.to_stream();
  server.handle_connection(con).unwrap();
  let data = stream.copy_written_data_to_string();
  assert_eq!(data, "HTTP/1.1 204 No Content\r\nConnection: Close\r\nContent-Length: 0\r\n\r\n");
}
