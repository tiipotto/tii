use std::net::{self, IpAddr, SocketAddr, ToSocketAddrs};

/// Returns the first loopback, if it exists, to the socket.
/// Useful for waking up threads in std-only environments.
pub fn unspecified_socket_to_loopback<S>(socket: S) -> Option<SocketAddr>
where
  S: ToSocketAddrs,
{
  let Ok(mut addrs) = socket.to_socket_addrs() else {
    return None;
  };
  let mut socket = addrs.next()?;
  specify_socket_to_loopback(&mut socket);
  Some(socket)
}

///
/// If a socket is unspecified (ex: 0.0.0.0) then its ip addr will be specified to localhost.
/// The port will be kept as is.
///
pub fn specify_socket_to_loopback(sock: &mut SocketAddr) {
  if sock.ip().is_unspecified() {
    match sock.ip() {
      IpAddr::V4(_) => sock.set_ip(IpAddr::V4(net::Ipv4Addr::new(127, 0, 0, 1))),
      IpAddr::V6(_) => sock.set_ip(IpAddr::V6(net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0x1))),
    };
  }
}
