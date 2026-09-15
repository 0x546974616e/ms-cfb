use std::{error, fmt};

pub type PodResult<T> = Result<T, PodError>;

#[derive(Debug, PartialEq, Eq)]
pub enum PodError {
  /// La séquence d'octets est plus petite que le type POD.
  SizeOfMismatch { length: usize, size_of: usize },

  /// La séquence d'octets n'est pas correctement alignée pour le POD.
  #[cfg(any(clippy, not(feature = "unaligned")))]
  AlignOfMismatch { pointer: usize, align_of: usize },
}

impl error::Error for PodError {}

impl fmt::Display for PodError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::SizeOfMismatch { length, size_of } => {
        write!(
          formatter,
          "Byte array is {} bytes, expected at least {} bytes.",
          length, size_of,
        )
      }

      #[cfg(any(clippy, not(feature = "unaligned")))]
      Self::AlignOfMismatch { pointer, align_of } => {
        write!(
          formatter,
          // Pour ne pas leak une adresse mémoire.
          "Byte array's pointer 0x******{:02X} is not aligned to {} bytes.",
          pointer % align_of,
          align_of,
        )
      }
    }
  }
}
