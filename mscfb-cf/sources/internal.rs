mod cache;
mod control;
mod core;
mod direntry;
mod error;
mod header;
mod mini;
mod object;
mod sector;
mod version;

pub use cache::*;
pub use control::*;
pub use core::*;
pub use direntry::*;
pub use error::*;
pub use header::*;
pub use mini::*;
pub use object::*;
pub use sector::*;
pub use version::*;

pub trait CfReader: AsRef<[u8]> {}
impl<T: AsRef<[u8]>> CfReader for T {}

#[cfg(feature = "sync")]
pub(crate) type CfRefCell<T> = mscfb_utils::MultiThreadedRefCell<T>;

#[cfg(not(feature = "sync"))]
pub(crate) type CfRefCell<T> = mscfb_utils::SingleThreadedRefCell<T>;

#[cfg(feature = "sync")]
pub(crate) type CfRc<T> = std::sync::Arc<T>;

#[cfg(not(feature = "sync"))]
pub(crate) type CfRc<T> = std::rc::Rc<T>;

#[macro_use]
pub(crate) mod macros {
  #[macro_export]
  macro_rules! if_pedantic {
    ( $($EXPR:stmt)* ) => {
      $(
        #[cfg(any(clippy, feature = "pedantic"))]
        $EXPR;
      )*
    };
  }
}

/// Interface pour le [`CfCore`] et le [`CfMiniCore`].
pub trait CfAnyCore {
  fn sector_size(&self) -> usize;
  fn next_sector_id(&self, sector_id: CfSectorId) -> CfResult<CfSectorId>;
  fn locate_sector(&self, sector_id: CfSectorId) -> CfResult<&[u8]>;
}

pub(crate) struct CfInternal<R: CfReader> {
  pub core: CfRc<CfCore<R>>,
  pub mini_core: CfRc<CfMiniCore<R>>,
  pub directory_stream: CfRc<CfDirectoryStream<R>>,
}

impl<R: CfReader> Clone for CfInternal<R> {
  fn clone(&self) -> Self {
    Self {
      core: self.core.clone(),
      mini_core: self.mini_core.clone(),
      directory_stream: self.directory_stream.clone(),
    }
  }
}
