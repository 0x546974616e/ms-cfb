use mscfb_pod::PodError;
use std::borrow::Cow;
use std::{error, fmt, io};

use super::CfSectorId;

pub type CfResult<T> = Result<T, CfError>;

pub type CfErrorMessage = Cow<'static, str>;

/// Erreurs possibles produites par l'ensemble de la crate.
///
/// ```rust
/// # use mscfb_cf::{CfError, CfErrorInfo};
/// let value = "happened";
///
/// assert_eq!(
///   "BadHeaderSignature: Something happened here",
///   CfError::new(
///     CfErrorInfo::BadHeaderSignature,
///     format!("Something {value} here")
///   )
///   .to_string()
/// );
/// ```
pub struct CfError {
  message: CfErrorMessage,
  kind: CfErrorInfo,
}

// TODO: Ajouter des valeurs aux énumérations.
#[derive(Debug)]
pub enum CfErrorInfo {
  IoError(io::Error),
  PodError(PodError),

  // Header
  BadHeaderSignature,
  BadHeaderClsid,
  BadMinorVersion,
  BadMajorVersion,
  BadByteOrder,
  BadSectorShift,
  BadMiniSectorShift,
  BadReservedField,
  BadNumberOfDirectorySectors,
  BadMiniStreamCutoffSize,

  // Directory Entry
  BadDirectoryName,
  BadDirectoryNameLength,
  BadObjectType(u8),
  BadColorFlag,
  BadChildId,
  BadDirectoryClsid,
  BadStartingSectorLocation,
  BadStreamSize,

  // NotFound
  OutOfRange(usize),
  SectorNotFound(CfSectorId),
  HeaderNotFound,
  DifatNotFound,
  ObjectNotFound,

  // Unaligned
  UnalignedHeader,
  UnalignedSector,
  UnalignedDifat,
  UnalignedFat,

  // Misc
  DirectoryNameTooLong,
  InvalidStorageObject,
  Other,
  Todo,
}

impl CfError {
  pub fn new(kind: CfErrorInfo, message: impl Into<CfErrorMessage>) -> Self {
    CfError {
      message: message.into(),
      kind,
    }
  }
}

impl From<CfErrorInfo> for CfError {
  fn from(kind: CfErrorInfo) -> Self {
    CfError::new(kind, "")
  }
}

impl From<io::Error> for CfError {
  fn from(error: io::Error) -> Self {
    Self::from(CfErrorInfo::IoError(error))
  }
}

impl From<PodError> for CfError {
  fn from(error: PodError) -> Self {
    Self::from(CfErrorInfo::PodError(error))
  }
}

impl error::Error for CfError {
  fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    match &self.kind {
      CfErrorInfo::IoError(error) => Some(error),
      CfErrorInfo::PodError(error) => Some(error),
      _ => None,
    }
  }
}

impl fmt::Debug for CfError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut error = formatter.debug_struct("CfError");

    if !self.message.is_empty() {
      error.field("message", &self.message);
    }

    error.field("kind", &self.kind).finish()
  }
}

impl fmt::Display for CfError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.kind {
      CfErrorInfo::IoError(error) => fmt::Display::fmt(error, formatter),
      CfErrorInfo::PodError(error) => fmt::Display::fmt(error, formatter),
      error => write!(formatter, "{error:?}"),
    }?;

    if !self.message.is_empty() {
      formatter.write_str(": ")?;
      fmt::Display::fmt(&self.message, formatter)?;
    }

    Ok(())
  }
}

#[macro_use]
pub(crate) mod macros {
  #[macro_export]
  macro_rules! bail {
    // `return Err(..)` ou  `Err(..)?`
    ($KIND:ident, $MESSAGE:literal) => {
      return Err(crate::CfError::new(
        crate::CfErrorInfo::$KIND, $MESSAGE))
    };

    ($KIND:ident, $($ARGUMENT:tt)+) => {
      return Err(crate::CfError::new(
        crate::CfErrorInfo::$KIND, format!($($ARGUMENT)+)))
    };

    ($EXPRESSION:expr, $MESSAGE:literal) => {
      return Err(crate::CfError::new($EXPRESSION, $MESSAGE))
    };

    ($EXPRESSION:expr, $($ARGUMENT:tt)+) => {
      return Err(crate::CfError::new($EXPRESSION, format!($($ARGUMENT)+)))
    };

    ($KIND:ident) => {
      return Err(crate::CfErrorInfo::$KIND.into())
    };

    ($EXPRESSION:expr) => {
      return Err($EXPRESSION.into())
    };
  }
}
