use std::fmt;
use std::io::Write;

#[macro_export]
macro_rules! snprintf {
  ( $BUFFER:expr, $($ARGUMENT:tt)* ) => {
    $crate::snprintf($BUFFER, format_args!($($ARGUMENT)*))
  };
}

// TODO: À renommer.
pub fn snprintf<'buffer, const N: usize>(
  buffer: &'buffer mut [u8; N],
  arguments: fmt::Arguments<'_>,
) -> &'buffer str {
  let mut slice = &mut buffer[..];
  if slice.write_fmt(arguments).is_ok() {
    let written = N - slice.len();

    // On convertir la partie écrite de manière safe (au cas où).
    if let Ok(string) = std::str::from_utf8(&buffer[..written]) {
      return string;
    }
  }

  "(error)"
}
