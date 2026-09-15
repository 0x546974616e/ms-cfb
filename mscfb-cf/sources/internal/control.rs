use mscfb_pod::Pod;
use std::marker::PhantomData;
use std::mem::size_of;

use super::{
  CfContiguousCache, CfCore, CfErrorInfo, CfRc, CfReader, CfRefCell, CfResult,
  CfSectorId, CfSectorIdChain,
};

/// **Stream** bidirectionel du **Compound File** avec des données internes.
///
/// Pourquoi bidirectionel ? Car ces données internes peuvent être accedées de
/// manière aléatoire par dessus la chaîne de secteurs. Ce type de *stream* est
/// utilsé pour stocker les [entrées du répertoire][super::CfDirectoryEntry], la
/// [Mini FAT][super::CfMiniFat] et le [Mini Stream][super::CfMiniStream].
pub struct CfControlStream<T: Pod, R: CfReader> {
  _marker: PhantomData<T>,
  pub cache: CfRefCell<CfContiguousCache<CfSectorIdChain<R>>>,
  element_per_sector: usize,
  core: CfRc<CfCore<R>>,
}

impl<T: Pod, R: CfReader> CfControlStream<T, R> {
  #[inline(always)]
  pub fn core(&self) -> CfRc<CfCore<R>> {
    self.core.clone()
  }

  pub fn new(sector_id: CfSectorId, core: impl Into<CfRc<CfCore<R>>>) -> Self {
    let core = core.into();

    // TODO (nightly): Il n'y pas a pas moyen de vérifier ça à la compilation.
    debug_assert!(core.version().sector_size().is_multiple_of(size_of::<T>()));
    let chain = CfSectorIdChain::new(sector_id, core.clone());

    CfControlStream {
      _marker: PhantomData,
      cache: CfContiguousCache::new(chain).into(),
      element_per_sector: core.version().sector_size() / size_of::<T>(),
      core,
    }
  }

  /// Récupère le n-ième élement dans la chaîne de secteur.
  pub fn nth_element(&self, index: usize) -> CfResult<&T> {
    // On cherche dans quel secteur se trouve l'élément.
    let nth_sector = index / self.element_per_sector;

    // TODO (async): Revoir l'INTERIOR MUTABILITY.
    let mut cache = self.cache.write();
    let sector_id = cache.nth_sector_id(nth_sector)?;

    // On va maintenant chercher l'élement dans le secteur.
    unsafe { self.core.locate_sector_as::<T>(sector_id) }?
      .get(index % self.element_per_sector)
      .ok_or(CfErrorInfo::OutOfRange(index).into())
  }
}
