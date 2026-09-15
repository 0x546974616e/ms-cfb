use std::fmt::{self, Write};

use super::snprintf;

const CHUNK: usize = 16;

pub fn hexdump(bytes: &[u8]) {
  hexdump_offset(bytes, 0usize);
}

pub fn ehexdump(bytes: &[u8]) {
  hexdump_offset(bytes, 0usize);
}

pub fn hexdump_offset(bytes: &[u8], offset: usize) {
  println!("{:#?}", HexDump::new(offset, bytes));
}

pub fn ehexdump_offset(bytes: &[u8], offset: usize) {
  eprintln!("{:#?}", HexDump::new(offset, bytes));
}

pub struct HexDump<T: AsRef<[u8]>> {
  offset: usize,
  bytes: T,
}

impl<T: AsRef<[u8]>> HexDump<T> {
  pub fn new(offset: usize, bytes: T) -> Self {
    HexDump { offset, bytes }
  }
}

impl<T: AsRef<[u8]>> From<T> for HexDump<T> {
  fn from(bytes: T) -> Self {
    HexDump::new(0, bytes)
  }
}

impl<T: AsRef<[u8]>> fmt::Display for HexDump<T> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&self, formatter)
  }
}

impl<T: AsRef<[u8]>> fmt::Debug for HexDump<T> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let bytes = self.bytes.as_ref();
    if !formatter.alternate() {
      return write!(formatter, "{:02X?}", bytes);
    }

    let mut buffer = [0x00u8; 64];
    let r#struct = snprintf!(&mut buffer, "[{} bytes]", bytes.len());
    let mut object = formatter.debug_struct(r#struct);

    let mut buffer = [0x00u8; 8];
    for (index, chunk) in bytes.chunks(CHUNK).enumerate() {
      let offset = index * CHUNK + self.offset;
      let field = snprintf!(&mut buffer, "{offset:08X}");
      object.field(field, &HexChunk(index, chunk));
    }

    object.finish()
  }
}

struct HexChunk<'bytes>(usize, &'bytes [u8]);

impl fmt::Debug for HexChunk<'_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let HexChunk(index, chunk) = *self;

    for byte in chunk {
      write!(formatter, "{byte:02X} ")?;
    }

    if chunk.len() < CHUNK && index > 0 {
      let missing = CHUNK.saturating_sub(chunk.len()) * 3;
      write!(formatter, "{:>missing$}", "")?;
    }

    for byte in chunk {
      formatter.write_char(
        // Est-ce qu'on affiche l'ASCII étendu (128..=255) ?
        if byte.is_ascii_graphic() {
          *byte as char
        } else {
          '.'
        },
      )?;
    }

    formatter.write_char(' ')
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hexdump_indent() {
    #[allow(unused)]
    #[derive(Debug)]
    struct Dada {
      dada: usize,
      fafa: HexDump<Vec<u8>>,
      gaga: bool,
    }

    let value = Dada {
      dada: 1337,
      gaga: false,
      fafa: HexDump::new(
        512, // Offset
        [
          0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, //
          0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, //
          b'C', b'o', b'u', b'C', b'o', b'u', b'u', b'u', //
          b'u', b'u', b'u', b'U', b'u', b'U', b'U', b'U', //
          0x42, 0x69, 0x13, 0x37, b'!',
        ]
        .into(),
      ),
    };

    assert_eq!(
      [
        "Dada {",
        "    dada: 1337,",
        "    fafa: [37 bytes] {",
        "        00000200: 00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F ................ ,",
        "        00000210: 43 6F 75 43 6F 75 75 75 75 75 75 55 75 55 55 55 CouCouuuuuuUuUUU ,",
        "        00000220: 42 69 13 37 21                                  Bi.7! ,",
        "    },",
        "    gaga: false,",
        "}",
        ].join("\n"),
        format!("{value:#?}"),
    );
  }
}
