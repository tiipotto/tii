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
  if socket.ip().is_unspecified() {
    match socket.ip() {
      IpAddr::V4(_) => socket.set_ip(IpAddr::V4(net::Ipv4Addr::new(127, 0, 0, 1))),
      IpAddr::V6(_) => socket.set_ip(IpAddr::V6(net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0x1))),
    };
  }
  Some(socket)
}
