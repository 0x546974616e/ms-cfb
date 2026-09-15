use std::cmp::Ordering;
use std::{fmt, io};

use crate::bail;
use crate::internal::CfUserDataStream;

use super::{
  CF_NOSTREAM, CfDirectoryEntry, CfDirectoryId, CfDirectoryStream, CfError,
  CfErrorInfo, CfInternal, CfMiniSectorChain, CfRc, CfReader, CfResult,
  CfSectorChain,
};

pub struct CfObject<R: CfReader> {
  directory_entry: CfDirectoryEntry,
  object_type: CfObjectType,
  internal: CfInternal<R>,
}

impl<R: CfReader> Clone for CfObject<R> {
  fn clone(&self) -> Self {
    Self {
      object_type: self.object_type,
      directory_entry: self.directory_entry,
      internal: self.internal.clone(),
    }
  }
}

impl<R: CfReader> CfObject<R> {
  #[inline(always)]
  pub fn r#type(&self) -> CfObjectType {
    self.object_type
  }

  #[inline(always)]
  pub fn directory_entry(&self) -> &CfDirectoryEntry {
    &self.directory_entry
  }

  pub(crate) fn new(
    entry: CfDirectoryEntry,
    internal: CfInternal<R>,
  ) -> CfResult<Self> {
    let object_type = entry.validate()?;
    if object_type.is_unallocated() {
      bail!(Todo, "is_unallocated");
    }

    Ok(CfObject {
      directory_entry: entry,
      object_type,
      internal,
    })
  }

  // Recherche une entrée dans le répertoire.
  pub fn locate_object(&self, directory_name: &str) -> CfResult<Self> {
    let object_type = self.directory_entry.object_type()?;
    if !object_type.is_storage() && !object_type.is_root_storage() {
      bail!(InvalidStorageObject, "Storage object is required");
    }

    // Anti-bêtise, au cas où.
    let entry = &self.directory_entry;
    if entry.child_id == CF_NOSTREAM {
      bail!(InvalidStorageObject);
    }

    // On va chercher l'entrée dans le répertoire.
    let entry = self
      .internal
      .directory_stream
      .locate_entry(entry.child_id, directory_name)?;

    // Peut-être que le répertoire est vide.
    let Some(entry) = entry else {
      bail!(ObjectNotFound);
    };

    // On demande un object valide/alloué.
    let object_type = entry.validate()?;
    if object_type.is_unallocated() {
      bail!(Todo, "is_unallocated");
    }

    Ok(Self {
      internal: self.internal.clone(),
      directory_entry: *entry,
      object_type,
    })
  }

  // Traverse le répertoire courant.
  pub fn walk_directory(
    &self,
    mut callback: impl Fn(&CfObject<R>),
  ) -> CfResult<()> {
    let object_type = self.directory_entry.object_type()?;
    if !object_type.is_storage() && !object_type.is_root_storage() {
      bail!(InvalidStorageObject, "Storage object is required");
    }

    // Anti-bêtise, au cas où.
    let entry = &self.directory_entry;
    if entry.child_id == CF_NOSTREAM {
      return Ok(());
    }

    self.internal.directory_stream.walk_directory(
      entry.child_id,
      &mut |entry| {
        // NOTE: Peut-être que y'a mieux à faire que `clone()` tout le
        // temps, mais à la fois, dans une premier temps, ça sera ok.
        callback(&CfObject {
          internal: self.internal.clone(),
          object_type: entry.object_type()?,
          directory_entry: *entry,
        });

        Ok(())
      },
    )
  }

  pub fn open_chain(&self) -> CfResult<CfUserDataStream<R>> {
    let entry = &self.directory_entry;
    if !self.object_type.is_stream() {
      bail!(Todo, "Stream object is required");
    }

    Ok(if entry.stream_size < 4096 {
      CfUserDataStream::from_mini(
        entry.stream_size,
        CfMiniSectorChain::new(
          entry.starting_sector,
          self.internal.mini_core.clone(),
        ),
      )
    } else {
      CfUserDataStream::from(
        entry.stream_size,
        CfSectorChain::new(
          // Sinon c'est une chaîne "standard".
          entry.starting_sector,
          self.internal.core.clone(),
        ),
      )
    })
  }
}

macro_rules! object_type {
  (
    $($VARIANT:ident = $VALUE:literal
      $LETTER:literal [$METHOD:ident]),* $(,)?
  ) => {
    #[derive(Debug, Copy, Clone)]
    pub enum CfObjectType {
      $( $VARIANT = $VALUE, )*
    }

    impl TryFrom<u8> for CfObjectType {
      type Error = CfError;

      fn try_from(value: u8) -> CfResult<Self> {
        Ok(match value {
          $( $VALUE => Self::$VARIANT, )*

          _ => {
            let error = CfErrorInfo::BadObjectType(value);
            return Err(error.into());
          }
        })
      }
    }

    impl CfObjectType {
      $(
        #[inline(always)]
        pub fn $METHOD(self) -> bool {
          matches!(self, Self::$VARIANT)
        }
      )*

      pub fn letter(self) -> char {
        match self {
          $( Self::$VARIANT => $LETTER, )*
        }
      }
    }
  };
}

object_type! {
  Unallocated = 0x00 'u' [is_unallocated],
  Storage     = 0x01 'd' [is_storage],
  Stream      = 0x02 '-' [is_stream],
  RootStorage = 0x05 'R' [is_root_storage],
}
