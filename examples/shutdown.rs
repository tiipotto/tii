fn main() {
  // empty on purpose, the CI tests that this file is here, showdown is no longer something that the library cares about.
  // the previous impl not great anyways (setting atomic boolean and connecting to localhost is not good, this can fail if there is port exhaustion)
  // To implement this properly one has to call shutdown syscall on the rawfd of the TcpListen on unix.
  // On windows its not possible with the sockets provided by the STL because they do not use overlapped (interruptible) WSOCK2 io.
  // Its my honest opinion that outsourcing this to the user is best. They can use a third party socket library that implements this properly
  // On windows and I can just call shutdown on the fd in my unix code.
}

#[test]
fn run() {
  main();
}
