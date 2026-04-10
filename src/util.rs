use libflate::gzip;
use std::io;
use std::io::Write;
use std::sync::LockResult;

fn do_abort() -> ! {
  #[cfg(feature = "backtrace")]
  {
    let bt = backtrace::Backtrace::new();
    crate::error_log!("A impossible state was reached by the program. Please file a bug report on https://github.com/tiipotto/tii. The program will terminate now. bt={:?}", bt);
    eprintln!("A impossible state was reached by the program. Please file a bug report on https://github.com/tiipotto/tii. The program will terminate now. bt={bt:?}");
    std::process::abort();
  }
  #[cfg(not(feature = "backtrace"))]
  unreachable!("A condition that should be unreachable was reached. Please enable the 'backtrace' feature on tii for more information and then file a bug report!");
}

/// Convert a panic message from a catch_unwind or ThreadHandle::join into a str and call the close with it.
#[cfg(feature = "extras")] //For now only used by extras feature.
pub fn panic_msg<X>(
  panic_message: Box<dyn std::any::Any + Send + 'static>,
  handler: impl FnOnce(&str) -> X,
) -> X {
  if let Some(msg) = panic_message.downcast_ref::<&'static str>() {
    handler(msg)
  } else if let Some(msg) = panic_message.downcast_ref::<String>() {
    handler(msg)
  } else {
    let dbg = format!("{panic_message:?}");
    handler(&dbg)
  }
}

pub fn unreachable() -> ! {
  do_abort()
}

pub fn unwrap_some<T>(some: Option<T>) -> T {
  if let Some(t) = some {
    return t;
  }

  do_abort();
}

pub fn unwrap_ok<T, E>(some: Result<T, E>) -> T {
  if let Ok(t) = some {
    return t;
  }

  do_abort();
}

pub fn unwrap_poison<T>(result: LockResult<T>) -> io::Result<T> {
  result.map_err(|_| io::Error::other("Poisoned Mutex"))
}

/// This utility function can be used to prevent a heap allocation just to do a start_with or match check.
/// TODO in must be fast code we would call unsafe functions here, but lets not do that yet...
pub fn ascii_to_lower_first_n<'a, const N: usize>(
  stack_buffer: &'a mut [u8; N],
  data: &'a str,
) -> &'a str {
  let data_bytes = data.as_bytes();
  let mut data_bytes = unwrap_some(data_bytes.get(..N.min(data_bytes.len())));
  while let Some(last_byte) = data_bytes.last().copied() {
    //strip trailing mb sequence, otherwise our friend str::from_utf8 may not have a great day.
    if last_byte & 0b1000_0000 == 0 {
      break;
    }

    data_bytes = unwrap_some(data_bytes.get(..data_bytes.len() - 1));
  }

  let stack_buffer = unwrap_some(stack_buffer.get_mut(..data_bytes.len()));
  stack_buffer.copy_from_slice(data_bytes);
  stack_buffer.make_ascii_lowercase();

  unwrap_ok(std::str::from_utf8(stack_buffer))
}

pub const fn three_digit_to_utf(num: u16) -> [u8; 3] {
  let n1 = num % 10;
  let n2 = ((num - n1) / 10) % 10;
  let n3 = (((num - n1) - n2) / 100) % 10;
  [b'0' + n3 as u8, b'0' + n2 as u8, b'0' + n1 as u8]
}

pub fn new_gzip_encoder<T: Write>(write: T) -> io::Result<gzip::Encoder<T>> {
  gzip::Encoder::with_options(
    write,
    gzip::EncodeOptions::default().header(gzip::HeaderBuilder::new().modification_time(0).finish()),
  )
}

#[cfg(not(target_has_atomic = "64"))]
mod counter {
  use std::sync::Mutex;
  use std::time::{SystemTime, UNIX_EPOCH};

  static COUNTER: Mutex<u128> = Mutex::new(0);

  pub fn next() -> u128 {
    let mut counter = COUNTER.lock().unwrap_or_else(|poison| {
      COUNTER.clear_poison();
      poison.into_inner()
    });

    if *counter == 0 {
      *counter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|a| a.as_millis())
        .unwrap_or_default()
        .checked_shl(64)
        .unwrap_or_default();
    }

    *counter += 1;
    *counter
  }
}

#[cfg(target_has_atomic = "64")]
mod counter {
  use std::sync::atomic::AtomicU64;
  use std::sync::atomic::Ordering;
  use std::time::{SystemTime, UNIX_EPOCH};

  static TIME: AtomicU64 = AtomicU64::new(0);
  static COUNTER: AtomicU64 = AtomicU64::new(0);
  pub fn next() -> u128 {
    let mut time = TIME.load(Ordering::Relaxed);
    if time == 0 {
      time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|a| a.as_millis() as u64)
        .unwrap_or_default();

      if let Err(t) = TIME.compare_exchange(0, time, Ordering::Relaxed, Ordering::Relaxed) {
        time = t;
      }
    }

    let time = u128::from(time).overflowing_shl(64).0;
    let counter = u128::from(COUNTER.fetch_add(1, Ordering::SeqCst));
    time | counter
  }
}

#[cfg(feature = "random_id")]
fn next_rand_id() -> u128 {
  let mut bytes = [0u8; 16];
  if getrandom::fill(&mut bytes).is_err() {
    return counter::next();
  }

  u128::from_ne_bytes(bytes)
}

pub fn next_id() -> u128 {
  #[cfg(feature = "random_id")]
  {
    next_rand_id()
  }

  #[cfg(not(feature = "random_id"))]
  {
    counter::next()
  }
}

#[cfg(feature = "log")]
#[macro_export]
///Calls trace!
macro_rules! trace_log {
    (target: $target:expr, $($arg:tt)+) => (log::log!(target: $target, log::Level::Trace, $($arg)+));
    ($($arg:tt)+) => (log::log!(log::Level::Trace, $($arg)+))
}

#[cfg(not(feature = "log"))]
#[macro_export]
///Calls trace!
macro_rules! trace_log {

  (target: $target:expr, $($arg:tt)+) => {
      let _ = &($($arg)+);
  };
  ($($arg:tt)+) => {
      let _ = &($($arg)+);
  }
}

#[cfg(feature = "log")]
#[macro_export]
///Calls debug!
macro_rules! debug_log {
    (target: $target:expr, $($arg:tt)+) => (log::log!(target: $target, log::Level::Info, $($arg)+));
    ($($arg:tt)+) => (log::log!(log::Level::Debug, $($arg)+))
}

#[cfg(not(feature = "log"))]
#[macro_export]
///Calls debug!
macro_rules! debug_log {

  (target: $target:expr, $($arg:tt)+) => {
      let _ = &($($arg)+);
  };
  ($($arg:tt)+) => {
      let _ = &($($arg)+);
  }
}

#[cfg(feature = "log")]
#[macro_export]
///Calls info!
macro_rules! info_log {
    (target: $target:expr, $($arg:tt)+) => (log::log!(target: $target, log::Level::Info, $($arg)+));
    ($($arg:tt)+) => (log::log!(log::Level::Info, $($arg)+))
}

#[cfg(not(feature = "log"))]
#[macro_export]
///Calls info!
macro_rules! info_log {

  (target: $target:expr, $($arg:tt)+) => {
      let _ = &($($arg)+);
  };
  ($($arg:tt)+) => {
      let _ = &($($arg)+);
  }
}

#[cfg(feature = "log")]
#[macro_export]
///Calls warn!
macro_rules! warn_log {
    (target: $target:expr, $($arg:tt)+) => (log::log!(target: $target, log::Level::Warn, $($arg)+));
    ($($arg:tt)+) => (log::log!(log::Level::Warn, $($arg)+))
}

#[cfg(not(feature = "log"))]
#[macro_export]
///Calls warn!
macro_rules! warn_log {

  (target: $target:expr, $($arg:tt)+) => {
      let _ = &($($arg)+);
  };
  ($($arg:tt)+) => {
      let _ = &($($arg)+);
  }
}

#[cfg(feature = "log")]
#[macro_export]
///Calls error!
macro_rules! error_log {
    (target: $target:expr, $($arg:tt)+) => (log::log!(target: $target, log::Level::Error, $($arg)+));
    ($($arg:tt)+) => (log::log!(log::Level::Error, $($arg)+))
}

#[cfg(not(feature = "log"))]
#[macro_export]
///Calls error!
macro_rules! error_log {

  (target: $target:expr, $($arg:tt)+) => {
      let _ = &($($arg)+);
  };
  ($($arg:tt)+) => {
      let _ = &($($arg)+);
  }
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDocTests;
