# tii
tii is fast compiling low-latency HTTP/1.1 web server, with support for static content and WebSockets.
tii is currently under active development.
While it does work, large amounts of breaking changes are expected.

## Quick Start
For a larger example that uses most features of tii see [basic](./examples/basic.rs). There are many more [examples](./examples).

```rust no_run
use std::net::TcpListener;
use std::time::Duration;

use tii::{MimeType, RequestContext, Response, ServerBuilder};

fn hello_world(request: &RequestContext) -> Response {
  let response_body = format!("Path: {} Hello, World!", request.request_head().get_path());
  Response::ok(response_body, MimeType::TextPlain)
}

fn main() {
  let tii_server = ServerBuilder::builder(|builder| {
    builder
      .router(|router| router.route_get("/*", hello_world))?
      .with_keep_alive_timeout(Some(Duration::ZERO)) //We disable http keep alive.
  })
  .unwrap();

  // This does not spawn any threads, everything will be done in the main thread!
  // Connections will be processed 1 at a time.
  let tcp_listen = TcpListener::bind("0.0.0.0:8080").unwrap();
  for tcp_stream in tcp_listen.incoming() {
    if let Err(err) = tii_server.handle_connection(tcp_stream.unwrap()) {
      eprintln!("Error handling request: {}", err);
    }
  }
}
```

## Goals
- Simplicity (simple source, but also simple to use)
- Low-latency
- Fast compile times (*1)
- Safety (unsafe_code is denied) (*2)

(*1) With default features. Some features, such as `rust-tls`, may increase compile times.

(*2) With default features. There are 2 unsafe blocks with the "extras" feature.
* 1 (Windows) call wsock WSAPoll to check if a TCPListener can accept a connection without blocking forever.
* 2 (Unix) call libc::shutdown on a TCPListener/UnixListener making it return from accept with an io error immediately.

Both of these are only needed because no alternative in the stdlib exists, should this change then those unsafe blocks will be removed.

## Non-goals
- [C10k problem](https://en.wikipedia.org/wiki/C10k_problem)
- thread pool
- async

  If you need to use async api's, something like
  [pollster](https://github.com/zesterer/pollster) is recommended.

## Architecture
The general Architecture of Tii is split into 3 parts that are designed to be used together
but can be used independent of each other. The layers use Traits that you can
implement yourself if you desire to replace any of them.

### Raw Connection
   * Stream oriented connection
   * Timeout+Buffering+Duplex
   * Implementation provided for TCP, Unix Socket, TCP+TLS, TLS+Unix Socket
   * Easily implementable by the user for any Stream Oriented connection that can do:
     * Full Duplex
     * Supports Timeouts
     * Supports Closing the connection or Re-Synchronizing the connection

The biggest difference from other HTTP Frameworks is that Tii does not bind a socket for you or accept connections.
The entrypoint to a `TiiServer` is the fn `handle_connection` which accepts an arbitrary Raw Connection and
will process it. All requests on that connection (if keep alive is enabled and supported this may be multiple)
will be processed completely in the caller thread. Tii itself will NOT spawn any threads or otherwise
move the processing to another thread.
A typical use of Tii would call `TcpListen::accept` and move the resulting `TcpStream`
into a pooled Thread where `TiiServer::handle_connection` is called.
TiiServer::handle_connection does not require `TiiServer` to be mut and can therefore be called on an `Arc` of `TiiServer`
by multiple Threads concurrently. It's up to the user of the library to decide what thread pool implementation (if any) to use.
A naive implementation can also just use `thread::spawn` instead of relying on a third party or self built thread pool.

Multi-threading is not strictly necessary as you could also just call `TcpListen::accept`
again after `handle_connection` returns and do everything in a single thread.
If you do this then keep in mind an Endpoint that blocks for a long time
will also block all other requests and Clients will eventually probably time out.
It is also recommended to disable Keep-Alive in such a scenario to make `handle_connection`
return faster instead of it only returning after the Keep-Alive grace expired.
This is attractive for very small applications running on very resource constrained environments
that do not need to handle concurrent requests and only desire a very small footprint.

`handle_connection` will return Ok if the connection is finished.
If Ok is returned then the caller is guaranteed that only syntactically correct well-formed HTTP
requests+responses have been processed by Tii and sent over the Raw Connection.

Tii will return Err to `handle_connection` in case of a fatal error.
Examples for Errors considered fatal for the connection:
* the bytes read from the raw connection do not conform to valid HTTP.
  * For example use of non ASCII characters in an HTTP method.
* IO Error
  * Read Timeout
  * Connection Reset
  * Unexpected EOF
    * If connection keep alive is enabled and the connection does EOF
    directly after the end of a well-formed http request, then this is not treated as an error.
* Processing further down the line returned an Err.
  * See below for more info

All Connections implemented by default are closed if `handle_connection` returns Ok or Err as the HTTP protocol generally assumed
that for example a `Connection: Close` will close the underlying socket. This is however not mandatory and
can be overwritten by users of the library that implement the Raw Connection and just pass the same connection again
to `handle_connection`. Obviously this makes little sense for TCP and Unix Sockets. This may make sense for other
byte stream connections such as for example a Serial Port connection. It cannot be closed and only be resynchronized.
Should you desire to use such an underlying Raw Connection then you are responsible for any resynchronization that is necessary
before you resubmit the connection to `handle_connection`. Also note that your underlying connection must make similar guarantees
as TCP in terms of data integrity. Any connection integrity is responsibility of the implementer of the Raw Connection.

### HTTP processing
   * Parsing of raw HTTP data into some usable rust struct model
   * Serializing of data into raw HTTP responses
   * Keep Alive (if enabled/supported)
   * Transfer Encoding

This part is pretty much standardized so there is no need for much flexibility here.

### Request routing
The most complex part of Tii.
Tii takes inspiration from Java's excellent Jetty Http Server
and the Java JAX-RS Web Application Standard but applies a rust spin to it.

When a request is received by HTTP processing and validated then it is passed to 1 or more `Router`s.
The `Router`s are processed in natural order for each request.
The first thing a router must do is decide if it should handle a request or not.
Once a router picks up a request it must handle it. If it decides to do so
all further routers no longer get called.

Should no router pick up a request then fallback behavior occurs that returns HTTP 404.
This can be changed by the user.

The intended purpose of this filtering is for the request to be roughly handled
by the correct part of your application. To do so you can evaluate any properties
of the HTTP Request aside from the request body. Typically, this would be used to
separate multiple virtual hosts based on the Host header, or evaluate the "base path"
of the request. (For Example all requests that start with /api go to the api `Router`,
all other requests are handled by the next `Router` which serves static files or yields 404)

Once a `Router` begins handling a request it must produce a Response.

Most single purpose applications will likely only have a single `Router`.

`Router` itself is a Trait so you can fully customize what a `Router` might do,
if you so desire.

Tii provides one `Router` implementation which does path, method and media type based
endpoint matching. This means you can register endpoints by path, method and media type to the `Router` and if
the request matches all desired criteria then your endpoint gets called and can produce a `Response`.

In addition to doing path based request matching the Default Tii Router also
allows for you to provide custom handling for Paths that have no endpoint,
(By default 404 is returned) Error handling, (By default 500 is returned)
and Pre-Request and After-Request handling common to all endpoints.

Pre request handling is done by adding a `RequestFilter` to a `Router`.
You can, if your pre request handler so decides, also abort the request handling and skip
invocation of the actual endpoint.
A common use case where this may be desired is to check for Authentication Headers
and abort the request if it is not authenticated.

Just like with the `Router`s themselves there can be multiple `RequestFilter`s within a router.
They are split in two categories:
1. Pre-Routing
2. After-Routing

Pre-Routing `RequestFilter`s are always invoked and may modify the path of the request to affect
the outcome of routing. This is useful for redirecting stuff to a different "path"
without having to register the same endpoint under multiple paths.

Post-Routing `RequestFilter`s are only invoked if after routing an endpoint to handle the request actually exists.
Post-Routing `RequestFilter`s cannot change the request path anymore.

If no endpoint exists then Post-Routing `RequestFilter`s are skipped
and handling jumps to the `NotFoundHandler`, which by default will return a 404 Response.

In both categories the filters are always processed in natural order. As soon as a `RequestFilter`
aborts the request with a response then filters and the endpoint are not invoked anymore.

After-Request handling is done by adding a `ResponseFilter` to a `Router`
Each response filter is always called exactly once per `Request` as soon as the `Response`
object has been created by either an endpoint or a `RequestFilter`

A `ResponseFilter` can fully modify every aspect of the Response including the ResponseBody, Headers and Status code.
It has access to all information from the `Request` except for possibly the RequestBody, since that might have already consumed by the endpoint or a `RequestFilter`.

A common use case for a `ResponseFilter` would be to add CORS headers or other Custom Headers
that should be added to every response.

Lastly there is error handling. By default, Endpoints, `NotFoundHandler`, `ResponseFilter`s, `RequestFilters`s
will be able to return an arbitrary Result. If the result is Err then Processing immediately skips to
the `ErrorHandler` of the `Router`. The `ErrorHandler` must produce a Response. By default, this just yields
HTTP 500 without any response body. The `ErrorHandler` has full access to the `Request` expect for the
possibly already consumed body and the Err value in the result which can be used in a Boxed version with downcast_ref.

The `ErrorHandler` itself may also return an Err value. This error is assumed fatal for the connection and Tii will NOT
write any bytes to the connection respond to such a request. The error is propagated all the way back to the user
of the library to the `handle_connection` fn which originally accepted the Raw Connection.
There the user of the library may decide to log such a fatal error.
In any case the connection should be closed or resynchronized in this case.
The default TCP/Unix Socket implementation closes the Socket in this case.

Should an `ErrorHandler` return an Ok with a Response then all remaining `ResponseFilter` will get called.
Should any further `ResponseFilter` invocation return an Err then the `ErrorHandler` will be called again.
`ResponseFilter`s that have already been called once for the request won't get called again to prevent infinite loops.

If an IO/Error occurs on the connection in relation to reading the RequestBody inside an Endpoint or Filter then the
connection is marked as tainted. Depending on how the endpoints deal with the error and how they propagate it
the error handlers might be called. The generated response is irrelevant in this scenario
and never written out as all IO/Errors on the underlying connection are assumed to be fatal.

If no user code (Filter/Endpoint/NotFoundHandler/ErrorHandler) fully consumes the request body then
Tii will consume and discard the entire RequestBody after writing the Response.
Any error that occurs during this is treated as a fatal error.
User code is free to read from the request body during writing of the ResponseBody.
This is useful for doing in place transformation of large data (such as video transcoding) where first reading
the entire source before beginning to produce an output is not acceptable.

## Use your own Thread Pool
By default, whenever tii needs to spawn a thread it uses `thread::Builder::new().spawn` to spawn a new thread.
Tii does not do or need this by default, only if you enable and use some features such as "rust-tls" or "extras" then
some functions will offer a way for you to pass a thread pool. It is very obvious to the user when this is the case.

Your custom thread pool will have to implement the tii::ThreadAdapter trait. This trait requires your implementation
to provide a single "spawn" function that runs the given task and returns some type of join handle.
It should not be hard or much effort to implement this trait for any given thread pool.

If your thread pool does not support join handles then you can return a noop join handle (`ThreadAdapterJoinHandle::default()`)
that does nothing when join() is called. The join handle is not used for critical synchronization or passing return values.
It is only used to ensure that resources are actually freed already and to log panics that may have occurred
in user code in the separate thread. If you do not care about immediate freeing of resources (they are still freed but just in the "background")
or logging panic messages then returning a noop join handle is acceptable.

The default "TheadAdapter" implementation looks like this:
```rust
use tii::{ThreadAdapter, TiiResult, ThreadAdapterJoinHandle};

#[derive(Debug)]
pub(crate) struct DefaultThreadAdapter;
impl ThreadAdapter for DefaultThreadAdapter {
  fn spawn(&self, task: Box<dyn FnOnce() + Send>) -> TiiResult<ThreadAdapterJoinHandle> {
    let hdl : std::thread::JoinHandle<()> = std::thread::Builder::new().spawn(task)?;
    Ok(ThreadAdapterJoinHandle::new(Box::new(move || hdl.join())))
  }
}
```

## Panics
Tii makes a valiant effort to avoid panics where possible.
It does not contain a single catch_unwind.
If an endpoint triggers a panic that panic is not caught by tii.
The user of the library may decide to catch the panic as it will eventually be propagated to
the caller of `handle_connection`.
In the unlikely scenario that Tii triggers a panic internally please create a bug report.
You can enable the `backtrace` feature to force Tii to print a full backtrace and
automatically call abort() to abort the process in some situations where presumed unreachable code was hit.
It is not recommended to use this feature in production
unless you are actively troubleshooting a problem.
Its advised in general to use panic=abort when working with tii outside of debugging/development.

## Extras
The `extras` namespace is for convenience over robustness and is always completely optional; think reusable examples/apps/templates, not staging.
Stability is not guaranteed (functionality may be moved to another crate or functionality added to Tii).

Nothing in Tii will ever depend on anything in `extras`.

## Special Thanks
- [Humphrey](https://github.com/w-henderson/Humphrey)
