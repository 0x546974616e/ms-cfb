use mscfb_pod::{Pod, PodError};

#[cfg(test)]
mod tests {
  use super::*;
  use std::mem::offset_of;

  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  struct Dada {
    a: u64,
    b: u32,
    c: u16,
    d: u8,
  }

  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  struct Fafa([u8; 3], Dada, u8);

  unsafe impl Pod for Dada {}
  unsafe impl Pod for Fafa {}

  #[test]
  fn size_of_error() {
    assert_eq!(size_of::<Dada>(), 16);
    assert_eq!(align_of::<Dada>(), 8);

    assert_eq!(
      unsafe { Dada::from_bytes(&[1, 2, 3]) },
      Err(PodError::SizeOfMismatch {
        size_of: 16,
        length: 3,
      }),
    );
  }

  #[test]
  #[cfg(any(clippy, not(feature = "unaligned")))]
  fn align_of_error() {
    assert_eq!(size_of::<Dada>(), 16);
    assert_eq!(align_of::<Dada>(), 8);

    let bytes: &[u8; 1 + 16] = &[
      0x00, // Pour être sûr que ça n'est pas aligné (?).
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
    ];

    let slice = &bytes[1..];
    assert_eq!(
      unsafe { Dada::from_bytes(slice) },
      Err(PodError::AlignOfMismatch {
        pointer: slice.as_ptr() as usize,
        align_of: 8,
      }),
    )
  }

  #[test]
  fn simple_struct() {
    assert_eq!(size_of::<Dada>(), 16);
    assert_eq!(align_of::<Dada>(), 8);
    assert_eq!(offset_of!(Dada, a), 0);
    assert_eq!(offset_of!(Dada, b), 8);
    assert_eq!(offset_of!(Dada, c), 12);
    assert_eq!(offset_of!(Dada, d), 14);

    const DADA: Dada = Dada {
      a: 0x04_04_04_04_04_04_04_04_u64,
      b: 0x03_03_03_03_u32,
      c: 0x02_02_u16,
      d: 0x01_u8,
    };

    #[rustfmt::skip]
    let bytes: [u8; 24] = [
      // De cette façon l'ordre des octets n'a pas de conséquence.
      0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, /* a */
      0x03, 0x03, 0x03, 0x03, /************************* b */
      /*********************/ 0x02, 0x02, /************* c */
      /*********************************/ 0x01, /******* d */
      /***************************************/ 0xAA, // Padding
      0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, // Le reste.
    ];

    assert_eq!(
      unsafe { Dada::from_bytes(&bytes) },
      Ok((&DADA, &[0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8][..])),
    );
  }

  #[test]
  fn nested_struct() {
    assert_eq!(size_of::<Fafa>(), 32);
    assert_eq!(align_of::<Fafa>(), 8);
    assert_eq!(offset_of!(Fafa, 0), 0);
    assert_eq!(offset_of!(Fafa, 1), 8);
    assert_eq!(offset_of!(Fafa, 2), 24);

    const DADA: Dada = Dada {
      a: 0x04_04_04_04_04_04_04_04_u64,
      b: 0x03_03_03_03_u32,
      c: 0x02_02_u16,
      d: 0x01_u8,
    };

    const FAFA: Fafa = Fafa([0x01, 0x02, 0x03], DADA, 0x42);

    #[rustfmt::skip]
    let bytes: [u8; 40] = [
      // On ne teste pas l'ordre des octets.
      0x01, 0x02, 0x03, /******************************* 0 */
      /***************/ 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
      0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, /* a */
      0x03, 0x03, 0x03, 0x03, /************************* b */
      /*********************/ 0x02, 0x02, /************* c */
      /*********************************/ 0x01, /******* d */
      /***************************************/ 0xAA, // Padding
      0x42, /******************************************* 2 */
      /***/ 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, // Padding
      0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, // Le reste.
    ];

    assert_eq!(
      unsafe { Fafa::from_bytes(&bytes) },
      Ok((&FAFA, &[0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8][..])),
    );
  }
}
