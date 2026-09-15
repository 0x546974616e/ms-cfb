#![doc = include_str!("../README.md")]
#![allow(clippy::collapsible_if)]
#![allow(unused)] // TODO TMP

mod internal;
use internal::{
  CfCore, CfDirectoryEntry, CfDirectoryStream, CfInternal, CfMiniCore,
  CfMiniFat, CfMiniStream, CfRc, CfReader,
};

pub use internal::{CfError, CfErrorInfo, CfObject, CfResult};
pub use memmap2::Mmap;
use std::path::Path;

pub struct CompoundFile<R: CfReader> {
  root_entry: CfObject<R>,
}

// TODO: Implémentation par très propre (pas de vraie gestion d'erreurs).
pub fn open(path: impl AsRef<Path>) -> CompoundFile<Mmap> {
  // 1. On ouvre le fichier (le Compound File).
  let file = std::fs::File::open(path).expect("fs");
  let mmap = unsafe { Mmap::map(&file).expect("mmap") };
  let core = CfRc::new(CfCore::new(mmap).expect("core"));

  // 2. On ouvre le chaîne de répertoires.
  let header = *unsafe { core.header() }.expect("header");
  let dirent = CfRc::new(CfDirectoryStream::new(
    header.first_directory_sector_id,
    core.clone(),
  ));

  // 3. On récupère le répertoire racine (`/`).
  let root_entry = *dirent.nth_element(0).expect("root");
  if !root_entry.object_type().expect("type").is_root_storage() {
    panic!("Bad root storage object.");
  }

  // 4. On initialise MiniStream, MiniFat et MiniCore.
  let mini_fat = CfMiniFat::new(header.first_mini_fat_sector_id, core.clone());
  let mini_stream = CfMiniStream::new(root_entry.starting_sector, core.clone());
  let mini_core = CfRc::new(CfMiniCore::new(mini_stream, mini_fat));

  // 5. On a tout ce qu'il faut pour faire le fichier.
  let internal = CfInternal {
    core: core.clone(),
    mini_core: mini_core.clone(),
    directory_stream: dirent,
  };

  // 6. Fin \o/
  CompoundFile {
    root_entry: CfObject::new(root_entry, internal).expect("root"),
  }
}

impl<R: CfReader> CompoundFile<R> {
  pub fn root(&self) -> &CfObject<R> {
    &self.root_entry
  }

  /// Ouvre un fichier dans le **Compound File**.
  pub fn open(&self, path: &str) -> CfResult<CfObject<R>> {
    // Dommage de `clone()` le root mais ok là.
    let mut object = self.root_entry.clone();

    for segment in path.split_terminator(['/', '\\']) {
      if !segment.is_empty() {
        object = object.locate_object(segment)?;
      }
    }

    Ok(object)
  }
}
