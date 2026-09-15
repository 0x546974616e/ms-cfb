use std::cmp::min;
use std::io;
use std::ptr::copy_nonoverlapping;

use crate::bail;

use super::{
  CfAnyCore, CfCore, CfErrorInfo, CfMiniCore, CfRc, CfReader, CfResult,
};

pub const CF_MAXREGSECT: u32 = 0xFFFFFFFA;
pub const CF_ENDOFCHAIN: u32 = 0xFFFFFFFE;

/// Numéro d'un secteur (*zero-based*).
///
/// Les secteurs sont énumérés simplement par leur ordre dans le fichier. Le
/// premier secteur contient [l'en-tête][super::CfHeader] du **Compound File**.
/// L'index d'un secteur est appelé **sector identifier** ou **sector number**
/// et commence à `#0` après l'en-tête.
///
/// ```txt
/// ┌───────────────────────────────────────────────────────────────────────┐
/// │            Compound File divided into equal-length sectors            │
/// ├───────────┬───────────┬───────────┬───────────┬───────────┬───────────┤
/// │ CF Header │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #N │
/// └───────────┴───────────┴───────────┴───────────┴───────────┴───────────┘
/// ```
pub type CfSectorId = u32;

pub type CfSectorIdChain<R> = CfAnySectorIdChain<CfCore<R>>;
pub type CfSectorChain<R> = CfAnySectorChain<CfCore<R>>;
pub type CfMiniSectorIdChain<R> = CfAnySectorIdChain<CfMiniCore<R>>;
pub type CfMiniSectorChain<R> = CfAnySectorChain<CfMiniCore<R>>;

mod inner {
  pub enum CfState<T> {
    Start,
    Current(T),
    End,
  }
}

/// Chaîne de numéros de secteur.
///
/// La logique est la même pour les [internal stream][CfSectorIdChain] ou les
/// [mini stream][CfMiniSectorIdChain]. La seule différence sont les numéros de
/// secteur réservés: au-dessus de [`MAXREGSECT`][CF_MAXREGSECT] pour le premier
/// et seulement [`ENDOFCHAIN`][CF_ENDOFCHAIN] pour le second.
///
/// ```txt
///       ╭╍╍╍╍╍╍╍╍╍╍╍╮                       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
/// ┌─────o─────┬─────▼─────┬───────────┬─────▼─────┬───────────┬─────┴─────┐
/// │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #4 | Sector #5 │
/// │  Chain #0 │  Chain #1 │  Whatever │  Chain #3 │  Chain #4 |  Chain #2 │
/// └───────────┴─────┬─────┴───────────┴─────┬─────┴─────▲─────┴─────▲─────┘
///                   │                       ╰╍╍╍╍╍╍╍╍╍╍╍╯           │
///                   ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
/// ```
pub struct CfAnySectorIdChain<C: CfAnyCore> {
  state: inner::CfState<CfSectorId>,
  first_sector_id: CfSectorId,
  core: CfRc<C>,
}

impl<C: CfAnyCore> CfAnySectorIdChain<C> {
  pub fn new(sector_id: CfSectorId, core: impl Into<CfRc<C>>) -> Self {
    CfAnySectorIdChain {
      state: inner::CfState::Start,
      first_sector_id: sector_id,
      core: core.into(),
    }
  }
}

impl<C: CfAnyCore> Iterator for CfAnySectorIdChain<C> {
  type Item = CfResult<CfSectorId>;

  fn next(&mut self) -> Option<Self::Item> {
    let next_sector_id = match self.state {
      // On commence tout juste la chaîne.
      inner::CfState::Start => self.first_sector_id,

      // Par construction, `sector_id` est toujours valide.
      inner::CfState::Current(sector_id) => {
        match self.core.next_sector_id(sector_id) {
          Ok(next_sector_id) => next_sector_id,

          Err(error) => {
            // On termine en cas d'erreur.
            self.state = inner::CfState::End;
            return Some(Err(error));
          }
        }
      }

      // Rien à faire, on est au bout.
      inner::CfState::End => return None,
    };

    // NOTE: La MiniFAT, contrairement à la FAT, n'a pas que la la valeur
    // `ENDOFCHAIN` (0xFFFFFFFE) de réservée. Mais dans un premier ça ne sera
    // pas grave de bloquer 6 valeurs après `MAXREGSECT` (0xFFFFFFFA) inclue.
    if next_sector_id < CF_MAXREGSECT {
      // C'est un sector_id valide, on continue.
      self.state = inner::CfState::Current(next_sector_id);
      return Some(Ok(next_sector_id));
    }

    // Sinon on termine la chaîne.
    self.state = inner::CfState::End;
    None
  }
}

/// Chaîne de secteurs (aussi appelée **stream**).
///
/// À la différence de la taille du secteur (**512** ou **4096** octets pour les
/// secteurs internes et **64** octets pour les minis secteurs), la logique est
/// la même pour les [internal stream][CfSectorIdChain] ou les [mini stream][
/// CfMiniSectorIdChain].
///
/// ```txt
///       ╭╍╍╍╍╍╍╍╍╍╍╍╮                       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
/// ┌─────o─────┬─────▼─────┬───────────┬─────▼─────┬───────────┬─────┴─────┐
/// │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #4 | Sector #5 │
/// │  Chain #0 │  Chain #1 │  Whatever │  Chain #3 │  Chain #4 |  Chain #2 │
/// └───────────┴─────┬─────┴───────────┴─────┬─────┴─────▲─────┴─────▲─────┘
///                   │                       ╰╍╍╍╍╍╍╍╍╍╍╍╯           │
///                   ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
/// ```
pub struct CfAnySectorChain<C: CfAnyCore> {
  chain: CfAnySectorIdChain<C>,
  remaining: Option<(CfSectorId, usize)>,
  core: CfRc<C>,
}

impl<C: CfAnyCore> io::Read for CfAnySectorChain<C> {
  fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
    match self.write(buffer) {
      Err(error) => Err(io::Error::other(error)),
      Ok(written) => Ok(written),
    }
  }
}

impl<C: CfAnyCore> CfAnySectorChain<C> {
  pub fn new(sector_id: CfSectorId, core: impl Into<CfRc<C>>) -> Self {
    let core = core.into();
    CfAnySectorChain {
      chain: CfAnySectorIdChain::new(sector_id, core.clone()),
      remaining: None,
      core,
    }
  }

  /// Copie un buffer vers un autre.
  ///
  /// # SAFETY
  ///
  /// Cette fonction fait comme [`slice::copy_from_slice()`] sauf qu'elle ne
  /// `panic!()` pas. Donc les mêmes avertissement s'appliquent mais à vos
  /// risques et périls (voir aussi [`std::ptr::copy_nonoverlapping()`]). La
  /// taille de la `source` et de la `destination` doivent être supérieures ou
  /// égales à la taille totale à copier ⚠️.
  ///
  /// ```txt
  /// ┌────────────────▼──────┐
  /// │ Source         ┆      │
  /// ├────────────────┆──────┴─────────┐
  /// │ Destination    ┆                │
  /// └────────────────▲────────────────┘
  ///                Count
  /// ```
  #[inline(always)]
  const unsafe fn copy(source: &[u8], destination: &mut [u8], count: usize) {
    debug_assert!(count <= source.len() && count <= destination.len());
    unsafe {
      copy_nonoverlapping(source.as_ptr(), destination.as_mut_ptr(), count)
    };
  }

  /// Copie les secteurs dans le buffer reçu.
  fn write(&mut self, mut buffer: &mut [u8]) -> CfResult<usize> {
    let mut total_written = 0usize;

    while !buffer.is_empty() {
      let (sector_id, offset) = {
        // S'il en restait du secteur précédent on continue.
        if let Some((sector_id, remaining)) = self.remaining {
          self.remaining = None;
          (sector_id, remaining)
        }
        // Sinon on s'attaque au prochain secteur.
        else {
          match self.chain.next().transpose()? {
            Some(sector_id) => (sector_id, 0usize),
            // Sauf si est arrivé au bout de la chaîne.
            None => return Ok(total_written),
          }
        }
      };

      let sector = self.core.locate_sector(sector_id)?;
      let sector = match sector.get(offset..) {
        None => bail!(CfErrorInfo::OutOfRange(offset)),
        Some(sector) => sector,
      };

      //  Cas n°1                      Cas n°2
      // ┌───────────▼                ┌───────────▼───────────┐
      // │  Buffer   ┆                │  Buffer   ┆    ...    │
      // ├───────────┆───────────┐    ├───────────┆───────────┘
      // │  Sector   ┆ Remaining │    │  Sector   ┆
      // └───────────▲───────────┘    └───────────▲
      // (Le cas n°3 se retrouve dans un des deux autres.)
      let written = min(sector.len(), buffer.len());
      let remaining = sector.len().saturating_sub(written);
      unsafe { Self::copy(sector, buffer, written) };
      total_written += written;

      if remaining != 0usize {
        self.remaining = Some((sector_id, remaining));
        // S'il nous en reste c'est que le buffer est plein (Cas n°1).
        return Ok(total_written);
      }

      // L'opérateur `[]` peut panic!(), j'aime pas ça.
      buffer = match buffer.get_mut(written..) {
        None => bail!(CfErrorInfo::OutOfRange(written)),
        Some(buffer) => buffer,
      };
    }

    Ok(total_written)
  }
}

/// Stream (chaîne de secteurs) dans le **Compound File** qui contient les
/// données de l'utilisateur (voir [`CfMiniSectorChain`] et [`CfSectorChain`]).
pub enum CfUserDataStream<R: CfReader> {
  MiniStream(io::Take<CfMiniSectorChain<R>>),
  Stream(io::Take<CfSectorChain<R>>),
}

impl<R: CfReader> CfUserDataStream<R> {
  pub fn from(stream_size: u64, chain: CfSectorChain<R>) -> Self {
    CfUserDataStream::Stream(io::Read::take(chain, stream_size))
  }

  pub fn from_mini(stream_size: u64, chain: CfMiniSectorChain<R>) -> Self {
    CfUserDataStream::MiniStream(io::Read::take(chain, stream_size))
  }
}

impl<R: CfReader> io::Read for CfUserDataStream<R> {
  fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
    match self {
      Self::MiniStream(stream) => stream.read(buffer),
      Self::Stream(stream) => stream.read(buffer),
    }
  }
}
