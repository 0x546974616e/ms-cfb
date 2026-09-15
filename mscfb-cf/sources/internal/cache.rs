use std::fmt;

use super::{CfError, CfErrorInfo, CfResult, CfSectorId};

/// Cache sur des numéros de secteur déjà visités.
///
/// Sauvegarde chaque [numéro de secteur][CfSectorId] rencontré dans un cache
/// local et permet un accès aléatoire direct à ces numéros en un temps
/// constant. En revanche, pour des grandes chaînes de secteur, l'utilisation
/// mémoire sera plus importante car chaque numéro est stocké indépendamment.
///
/// ```txt
///       ╭╍╍╍╍╍╍╍╍╍╍╍╮                       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
/// ┌─────o─────┬─────▼─────┬───────────┬─────▼─────┬───────────┬─────┴─────┐
/// │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #4 | Sector #5 │
/// │  Chain #0 │  Chain #1 │  Whatever │  Chain #3 │  Chain #4 |  Chain #2 │
/// └───────────┴─────┬─────┴───────────┴─────┬─────┴─────▲─────┴─────▲─────┘
///                   │                       ╰╍╍╍╍╍╍╍╍╍╍╍╯           │
///                   ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
///
/// Cache = [ 0, 1, 5, 3, 4 ]
/// ```
#[derive(Debug)]
pub struct CfLinearCache {
  cache: Vec<CfSectorId>,
}

impl CfLinearCache {
  pub fn new() -> Self {
    CfLinearCache {
      cache: Vec::<CfSectorId>::new(),
    }
  }

  /// Récupère le n-ième numéro de secteur de la chaîne.
  pub fn nth_sector_id(
    &mut self,
    index: usize,
    mut next_sector_id: impl FnMut() -> CfResult<Option<CfSectorId>>,
  ) -> CfResult<CfSectorId> {
    match self.cache.get(index) {
      // Si on l'a déjà en cache alors parfait.
      Some(sector_id) => Ok(*sector_id),

      None => {
        // Sinon on va le chercher.
        while index >= self.cache.len() {
          match next_sector_id()? {
            Some(sector_id) => self.cache.push(sector_id),
            None => break,
          };
        }

        let maybe = self.cache.get(index);
        // Mais peut-être qu'il n'existe pas.
        maybe.ok_or(CfErrorInfo::OutOfRange(index).into()).copied()
      }
    }
  }
}

/// Cache prenant en compte les numéros de secteur contigu.
///
/// En pratique, il y a beaucoup de secteurs contigus dans une chaîne. Dans
/// ce cas, plutôt que de stocker chaque [numéro de secteur][CfSectorId]
/// indépendamment, on va stocker des intervalles de numéros contigus.
///
/// Le principale avantage est une réduction de l'utilisation de la mémoire pour
/// des chaînes avec des grands intervalles de secteurs contigus. Le cache
/// stocke ces intervalles dans un vecteur avec les indices correspondant,
/// l'accès se fera donc en temps logarithmique (`O(log n)`). Par conséquent,
/// pour des chaînes trop éparses et trop peu contiguës, l'accès aléatoire pour
/// des numéros de secteur déjà visités sera plus long que le
/// [cache linéaire][CfLinearCache] (`O(1)`).
///
/// ```txt
///       ╭╍╍╍╍╍╍╍╍╍╍╍╮           ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
/// ┌─────o─────┬─────▼─────┬─────▼─────┬───────────┬───────────┬─────┴─────┐
/// │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #4 | Sector #5 │
/// │  Chain #0 │  Chain #1 │  Chain #3 │  Chain #4 │  Chain #5 |  Chain #2 │
/// └───────────┴─────┬─────┴─────┬─────┴───▲───┬───┴─────▲─────┴─────▲─────┘
///                   │           ╰╍╍╍╍╍╍╍╍╍╯   ╰╍╍╍╍╍╍╍╍╍╯           │
///                   ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
///
/// Cache = [ (0..=1), 5, (2..=4) ]
/// ```
pub struct CfContiguousCache<I> {
  /// Chaque élément du vecteur stocke des intervalles de numéros de secteur
  /// contigu en fonction de leur position dans la chaîne de secteurs qu'ils
  /// composent.
  ///
  /// ```txt
  /// ┌───────────┬───────────┬───────────┬───────────┐
  /// │   0..=5   │   6..=12  │  13..=13  │  14..=23  │ Nth sector
  /// ├───────────┼───────────┼───────────┼───────────┤
  /// │ 420..=425 │  92..=98  │ 102..=102 │ 410..=419 │ Sector ID
  /// └───────────┴───────────┴───────────┴───────────┘
  /// ```
  ///
  /// Pourquoi un vecteur et pas un arbre ? Dans un premier temps, un vecteur
  /// avec une recherche dichotomique suffira. En pratique les intervalles de
  /// secteurs contigus sont grands. Plus tard, pour une gestion de la mémoire
  /// plus intelligente, peut-être qu'un [`std::collections::BTreeMap`] ou une
  /// arène maison serait intéressant.
  cache: Vec<CfRangeInclusive>,
  total_index: usize,
  sector_ids: I,
}

impl<I> fmt::Debug for CfContiguousCache<I> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("CfContiguousCache")
      .field("total_index", &self.total_index)
      .field("cache", &self.cache)
      .finish()
  }
}

impl<I> fmt::Display for CfContiguousCache<I> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    use mscfb_utils::InBytes;
    use std::mem::size_of;

    #[derive(Debug)]
    #[allow(unused)]
    struct Estimed {
      r#type: &'static str,
      length: usize,
      element: InBytes<usize>,
      total: InBytes<usize>,
    }

    #[derive(Debug)]
    #[allow(unused, non_snake_case)]
    struct Addressable {
      V3: InBytes<usize>,
      V4: InBytes<usize>,
      mini: InBytes<usize>,
    }

    formatter
      .debug_struct("Cache")
      .field(
        "CfLinearCache",
        &Estimed {
          r#type: "Vec<CfSectorId>",
          length: self.total_index,
          element: size_of::<CfSectorId>().into(),
          total: InBytes::from(
            // On affiche ce qu'aurait pris un vecteur.
            self.total_index * size_of::<CfSectorId>(),
          ),
        },
      )
      .field(
        "CfContiguousCache",
        &Estimed {
          r#type: "Vec<CfRangeInclusive>",
          length: self.cache.len(),
          element: size_of::<CfRangeInclusive>().into(),
          total: InBytes::from(
            // Alors qu'ici on a justement compacté en range contigu.
            self.cache.len() * size_of::<CfRangeInclusive>(),
          ),
        },
      )
      .field(
        "_",
        &Addressable {
          V3: (self.total_index * 512).into(),
          V4: (self.total_index * 4096).into(),
          mini: (self.total_index * 64).into(),
        },
      )
      .finish()
  }
}

impl<I: Iterator<Item = CfResult<CfSectorId>>> CfContiguousCache<I> {
  pub fn new(iterator: I) -> Self {
    CfContiguousCache {
      sector_ids: iterator,
      total_index: 0usize,
      cache: vec![],
    }
  }

  /// Retourne le numéro de secteur si on l'a déjà.
  fn nth_cache(&self, index: usize) -> Option<CfSectorId> {
    if index >= self.total_index || self.cache.is_empty() {
      // On est sûr de pas l'avoir.
      return None;
    }

    // Recherche dichotomique (binary search).
    let mut left = 0usize;
    let mut right = self.cache.len() - 1;

    while left <= right {
      let middle = (left + right) / 2;
      // SAFETY: Par construction, l'élément existe.
      let range = unsafe { self.cache.get_unchecked(middle) };

      // Left          Middle         Right
      // ├╌╌╌╌╌╌╌╌╌╌╌┼────────┼━━━━━━━━━━━┤
      // ├╌╌╌╌╌╌╌╌╌╌╌┼────────┼━━━━━━━━━━━┤
      //           Start     End ^^^^^^^^
      if range.end_index < index {
        left = middle + 1;
      }
      // Left          Middle         Right
      // ├━━━━━━━━━━━┼────────┼╌╌╌╌╌╌╌╌╌╌╌┤
      // ├━━━━━━━━━━━┼────────┼╌╌╌╌╌╌╌╌╌╌╌┤
      //   ^^^^^^^ Start     End
      else if index < range.start_index {
        right = middle - 1;
      }
      // Left          Middle         Right
      // ├╌╌╌╌╌╌╌╌╌╌╌┼━━━━━━━━┼╌╌╌╌╌╌╌╌╌╌╌┤
      // ├╌╌╌╌╌╌╌╌╌╌╌┼━━━━━━━━┼╌╌╌╌╌╌╌╌╌╌╌┤
      //           Start ^^^ End
      else {
        debug_assert!(range.start_index <= index && index <= range.end_index);
        let position = index.saturating_sub(range.start_index);
        return Some(range.start_sector_id + position as u32);
      }
    }

    None
  }

  /// Ajoute le secteur à la fin.
  fn extend_cache(&mut self, new_sector_id: CfSectorId) {
    // On modifie le dernier intervalle s'ils sont contigus.
    if let Some(last_range) = self.cache.last_mut() {
      debug_assert!(last_range.end_index + 1 == self.total_index);
      if last_range.end_sector_id + 1 == new_sector_id {
        last_range.extend_range();
        self.total_index += 1;
        return;
      }
    }

    // Sinon il faut créer un nouvel élément.
    let new_range = CfRangeInclusive::new(self.total_index, new_sector_id);
    self.cache.push(new_range);
    self.total_index += 1;
  }

  /// Récupère le n-ième numéro de secteur de la chaîne.
  pub fn nth_sector_id(&mut self, index: usize) -> CfResult<CfSectorId> {
    // Soit on a le numéro de secteur.
    if let Some(sector_id) = self.nth_cache(index) {
      return Ok(sector_id);
    }

    // Sinon on va le chercher.
    while index >= self.total_index {
      match self.sector_ids.next().transpose()? {
        None => break,
        Some(new_sector_id) => {
          self.extend_cache(new_sector_id);
        }
      }
    }

    // Si on arrive là, c'est nécessairement le dernier.
    if let Some(last_range) = self.cache.last() {
      if last_range.end_index == index {
        return Ok(last_range.end_sector_id);
      }
    }

    // Où alors l'index demandé n'existe pas.
    Err(CfErrorInfo::OutOfRange(index).into())
  }
}

/// Intervalle de numéros de secteur avec leur position.
///
/// Le [`RangeInclusive`][std::ops::RangeInclusive] avec un tuple `(usize,
/// CfSectorId)` fait **40** octets. Le mauvais agencement des membres avec
/// l'alignement des `usize` augmentent la taille de la structure. On
/// préfèrera une structure maison.
///
/// ```rust
/// # type CfSectorId = u32;
/// # use std::{mem::size_of, ops::RangeInclusive};
/// # type CfRangeInclusive = (usize, usize, CfSectorId, CfSectorId);
/// assert_eq!(size_of::<RangeInclusive<(usize, CfSectorId)>>(), 40);
/// assert_eq!(size_of::<CfRangeInclusive>(), 24);
/// ```
struct CfRangeInclusive {
  // Est-ce qu'on accepte un `u32` ?
  start_index: usize,
  end_index: usize,

  // Attention à l'alignement.
  start_sector_id: CfSectorId,
  end_sector_id: CfSectorId,
}

impl fmt::Debug for CfRangeInclusive {
  fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    write!(
      formatter,
      "{}/{}..={}/{}[{}]",
      self.start_index,
      self.start_sector_id,
      self.end_index,
      self.end_sector_id,
      self.end_index - self.start_index + 1,
    )
  }
}

impl CfRangeInclusive {
  fn new(index: usize, sector_id: CfSectorId) -> Self {
    CfRangeInclusive {
      start_index: index,
      end_index: index,
      start_sector_id: sector_id,
      end_sector_id: sector_id,
    }
  }

  fn extend_range(&mut self) {
    self.end_sector_id += 1;
    self.end_index += 1;
  }
}
