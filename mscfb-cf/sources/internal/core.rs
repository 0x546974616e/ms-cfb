use mscfb_pod::Pod;
use std::mem::size_of;
use std::ops::Range;

use crate::{bail, if_pedantic};

use super::{
  CfAnyCore, CfError, CfErrorInfo, CfHeader, CfReader, CfResult, CfSectorId,
  CfVersion,
};

/// Alias d'un [numéro de secteur][super::CfSectorId] pour les secteurs FAT.
pub type CfFatId = CfSectorId;

/// Alias d'un [numéro de secteur][super::CfSectorId] pour les secteurs DIFAT.
pub type CfDifatId = CfSectorId;

/// Première abstraction d'un **Compound File**.
///
/// Seuls les accès à l'en-tête et aux secteurs sont implémentés ici. Cette
/// structure gère donc la DIFAT et la FAT.
///
/// ```txt
/// ┌───────────────────────────────────────────────────────────────────────┐
/// │            Compound File divided into equal-length sectors            │
/// ├─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┤
/// │ Hdr │ S#0 │ S#1 │ S#2 │ S#3 │ S#4 │ S#5 │ S#6 │ S#7 │ S#8 │ S#9 │ S#N │
/// └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘
///    ┌──┐           ┌─────┐           ┌─────┐
///    │  ├╍╍╍╍╍╍╍╍╍╍╍▶ S#2 ├╍╍╍╍╍╍╍╍╍╍╍▶ S#5 │ DIFAT
///    └──┘     ┌─────┼─────┘           └─────┘     ┌─────┐
///             │ S#1 ├╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍▶ S#7 │ FAT
///       ┌─────┼─────┘     ┌─────┬─────┐           └─────┘           ┌─────┐
/// Chain │ S#0 ├╍╍╍╍╍╍╍╍╍╍╍▶ S#3 │ S#4 ├╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍▶ S#N │
///       └─────┘           └─────┴─────┘     ┌─────┐     ┌─────┬─────┼─────┘
///                                     Chain │ S#6 ├╍╍╍╍╍▶ S#8 │ S#9 │
///                                           └─────┘     └─────┴─────┘
/// ```
///
/// Le **Compound File** est divisé en secteur de même taille
/// ([**512** ou **4096**][super::CfVersion::sector_size]). Les secteurs qui
/// sont chainés entre eux forment une chaîne de secteur et donc un *stream*. La
/// FAT permet de [reconstituer][Self::next_sector_id] les
/// [chaînes de secteurs][super::CfSectorIdChain] et la DIFAT permet de
/// [trouver les secteurs FAT][Self::nth_fat] dans le fichier.
///
/// Il y a quatre types de secteurs
///
/// - les [secteurs DIFAT][super::CfDifatId],
/// - les [secteurs FAT][super::CfFatId],
/// - les [secteurs "normaux"][super::CfAnySectorIdChain] (contenant les données
///   finales, soit les données non internes),
/// - et les secteurs libres (non utilisés).
///
/// Et il y a quatre types de *stream* :
///
/// - celui contenant tous les [répertoires][super::CfDirectoryEntry]
///   sous-jacent du **Compound File**,
/// - un stream pour la [mini FAT][super::CfMiniFat],
/// - un autre pour le [mini stream][super::CfMiniStream],
/// - et un dernier pour les données finales de l'utilisateur (les données non
///   internes aux **Compound File**).
///
/// Si on prend seulement la "première" DIFAT (les 109 premières entrées) on a
/// alors :
///
/// - en version 3, il y a **128** entrées par FAT avec des secteurs de **512**
///   octets, il donc est possible d'adresser en temps constant **109 × 128**
///   secteurs soit **109 × 128 × 512 = 7 143 424** octets (ou **7** Mo);
/// - en version 4, c'est **1024** entrées par FAT avec des secteurs de **4096**
///   octets donc **457 179 136** octets adressable (ou **457** Mo).
///
/// Si on rajoute un secteur DIFAT en plus, on multiplie les résultats
/// précédents par **127** en version 3 et **1023** en version 4 (voir
/// [`nth_fat()`][Self::nth_fat]). On se rend alors compte que quelques DIFAT
/// suffisent pour stocker des grandes quantités de données.
pub struct CfCore<R: CfReader> {
  underlying_file: R,
  version: CfVersion,
}

impl<R: CfReader> CfCore<R> {
  #[inline(always)]
  pub fn bytes(&self) -> &[u8] {
    self.underlying_file.as_ref()
  }

  #[inline(always)]
  pub fn version(&self) -> CfVersion {
    self.version
  }

  /// Crée puis vérifie l'en-tête selon la spécification (voir [MS-CFB]).
  ///
  /// [MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/53989ce4-7b05-4f8d-829b-d08d6148375b
  pub fn new(file: R) -> CfResult<Self> {
    let mut core = CfCore {
      underlying_file: file,
      version: CfVersion::V3,
    };

    // Bof mais osef `header()` cherche à l'offset `0` et ne dépend pas du
    // SECTOR_SIZE. Cependant, au cas où, on a mis 512 car c'est le minimum.
    let version = unsafe { core.header() }?.validate()?;
    core.version = version;
    Ok(core)
  }

  /// Retourne l'en-tête du fichier telle quelle.
  ///
  /// L'[en-tête][super::CfHeader] retournée n'inclue pas les 109 premières
  /// entrées de la DIFAT. C'est un choix de cette implémentation, c'est
  /// [`difat_header()`][Self::difat_header] qui a cette responsabilité-là. Une
  /// erreur est retournée si l'en-tête ne peut pas être lue ou si elle est mal
  /// alignée.
  ///
  /// # SAFETY
  ///
  /// L'[en-tête][super::CfHeader] est lue telle quelle depuis le fichier en
  /// mémoire. Le type est POD, aucun UB n'est attendu de ce côté-là. Attention
  /// par contre, l'endianness n'est donc pas pris en compte ⚠️.
  pub unsafe fn header(&self) -> CfResult<&CfHeader> {
    match self.bytes().get(..size_of::<CfHeader>()) {
      None => bail!(CfErrorInfo::HeaderNotFound),

      Some(bytes) => {
        let header = bytes.as_ptr().cast::<CfHeader>();
        if_pedantic! {
          if !header.is_aligned() {
            bail!(CfErrorInfo::UnalignedHeader);
          }
        }

        // On a mis `sizeof(CfHeader)`, il ne peut pas y avoir de reste.
        Ok(unsafe { header.as_ref_unchecked() })
      }
    }
  }

  /// Retourne les 109 premières entrées de la DIFAT contenues dans l'en-tête.
  ///
  /// Pourquoi 109 ? Car la taille d'un secteur en version 3 est de **512**
  /// octets et l'[en-tête][super::CfHeader] a une taille constance de **76**
  /// octets laissant alors **512 - 76 = 436** octets soit **109** entiers de 4
  /// octets. En version 4, les 3584 octets restants doivent être à zéro.
  ///
  /// Une erreur est retournée si les 109 premières entrées de la DIFAT ne
  /// peuvent pas être lues dans le premier secteur (dans l'en-tête) ou si elles
  /// ne sont pas correctement alignées.
  ///
  /// # SAFETY
  ///
  /// Cette DIFAT est directement réinterprétée depuis le fichier sous-jacent.
  /// Étant donné la nature du type (`[u32]`), aucun *undefined behavior* n'est
  /// à surprendre ici. Attention cependant, l'endianness n'est donc pas pris en
  /// compte ⚠️.
  pub unsafe fn difat_header(&self) -> CfResult<&[u32; 109]> {
    match self.bytes().get(size_of::<CfHeader>()..512) {
      None => bail!(CfErrorInfo::DifatNotFound),

      Some(bytes) => {
        // Toujours vrai, c'est vraiment pour du debug.
        debug_assert!(size_of::<CfHeader>() == 76usize);
        debug_assert!(bytes.len() == 512 - size_of::<CfHeader>());
        debug_assert!(bytes.len() == 109 * size_of::<u32>());

        // TODO (nightly): il y a `MaybeUninit::array_assume_init()`.
        let difat = bytes.as_ptr().cast::<[u32; 109]>();
        if_pedantic! {
          if !difat.is_aligned() {
            bail!(CfErrorInfo::UnalignedDifat);
          }
        }

        // On est forcément sur un tableau de 109 u32.
        // Au vu des constantes, ça ne peut pas être autrement.
        Ok(unsafe { difat.as_ref_unchecked() })
      }
    }
  }

  /// Retourne la position physique d'un secteur dans le fichier.
  ///
  /// Un **Compound File** est divisé en secteurs de tailles égales. Le premier
  /// secteur est [l'en-tête][super::CfHeader] du fichier et les numéros de
  /// secteur [`CfSectorId`] commencent après cette en-tête à partir l'index
  /// `#0`.
  ///
  /// En théorie un **Compound File** permet n'importe quelle taille de secteur
  /// (voir le **sector shift** dans [l'en-tête][super::CfHeader]). Mais en
  /// pratique la spécification impose une taille de **512** octets en version 3
  /// et **4096** en version 4.
  #[inline(always)]
  pub fn sector_offset(&self, sector_id: CfSectorId) -> usize {
    self
      .version
      .sector_size()
      .saturating_mul(sector_id as usize + 1)
  }

  /// Retourne l'intervalle physique d'un secteur dans le fichier.
  ///
  /// Le secteur `#0` est le premier après celui de [l'en-tête][super::CfHeader]
  /// du fichier. La taille d'un secteur est de **512** octets en version 3 et
  /// **4096** en version 4.
  #[inline(always)]
  pub fn sector_range(&self, sector_id: CfSectorId) -> Range<usize> {
    let sector_offset = self.sector_offset(sector_id);
    sector_offset..(sector_offset + self.version.sector_size())
  }

  /// Récupère un secteur à partir de son [identifiant][super::CfSectorId].
  ///
  /// Le décompte des [**sector identifer**][super::CfSectorId] commence à
  /// l'index `#0` après [l'en-tête][super::CfHeader] du fichier. La taille d'un
  /// secteur dépend de la version : **512** octets en version 3 et **4096** en
  /// version 4.
  ///
  /// Une erreur [`UnalignedSector`][super::CfErrorInfo::UnalignedSector] est
  /// retournée si le secteur n'est pas aligné par rapport à sa taille (dépend
  /// de la version).
  ///
  /// ```txt
  /// ┌───────────────────────────────────────────────────────────────────────┐
  /// │            Compound File divided into equal-length sectors            │
  /// ├───────────┬───────────┬───────────┬───────────┬───────────┬───────────┤
  /// │ CF Header │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #N │
  /// └───────────┴───────────┴───────────┴───────────┴───────────┴───────────┘
  /// ```
  pub fn locate_sector(&self, sector_id: CfSectorId) -> CfResult<&[u8]> {
    match self.bytes().get(self.sector_range(sector_id)) {
      None => bail!(CfErrorInfo::SectorNotFound(sector_id)),

      Some(sector) => {
        let _sector_size = self.version.sector_size();
        if_pedantic! {
          // TODO (nightly): ptr.is_aligned_to() est unstable (à l'août 2026).
          if !(sector.as_ptr() as usize).is_multiple_of(_sector_size) {
            bail!(CfErrorInfo::UnalignedSector);
          }
        }

        // Assertion de conscience tranquille j'avoue.
        debug_assert!(sector.len() == _sector_size);
        Ok(sector)
      }
    }
  }

  /// Récupère un secteur sous forme de `[T]` (typiquement `[u32]`).
  ///
  /// Si le secteur n'est pas aligné correctement par rapport à `T` alors
  /// l'erreur [`UnalignedSector`][super::CfErrorInfo::UnalignedSector] est
  /// retournée. Voir [`locate_sector()`][Self::locate_sector] pour plus de
  /// détails.
  ///
  /// # SAFETY
  ///
  /// Réinterprète le secteur comme un *slice* d'un autre type. Tous les
  /// avertissements liés à [`transmute()`][std::mem::transmute] s'appliquent
  /// ici. L'endianness n'est donc pas pris en compte ⚠️.
  pub unsafe fn locate_sector_as<T: Pod>(
    &self,
    sector_id: CfSectorId,
  ) -> CfResult<&[T]> {
    let sector = self.locate_sector(sector_id)?;
    let (_prefix, sector, _suffix) = unsafe { sector.align_to::<T>() };

    if_pedantic! {
      // On impose que le secteur soit aligné par rapport à `T`.
      // Et on ne tolère pas un reste à la fin du secteur.
      if !_prefix.is_empty() || !_suffix.is_empty() {
        bail!(CfErrorInfo::UnalignedSector);
      }
    }

    Ok(sector)
  }

  /// Retourne le [numéro de secteur FAT][super::CfFatId] contenant l'entrée
  /// donnée.
  ///
  /// Si on reconstruit la FAT de manière logique/contiguë alors c'est un simple
  /// tableau de dimension 1. Chaque indice `n` du tableau représente le
  /// `n`-ième numéro de secteur dans le fichier et chaque valeur à cet indice
  /// correspond au prochain numéro secteur d'une chaîne de secteur. La valeur
  /// magique `ENDOFCHAIN` (`0xFFFFFFFE`) indique la fin de la chaîne.
  ///
  /// Vue logique de la FAT :
  ///
  /// ```txt
  /// ┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
  /// │  FAT #0  │  FAT #1  │  FAT #2  │  FAT #3  │  FAT #4  │  FAT #N  │
  /// └──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
  /// ```
  ///
  /// Vue sur disque (la FAT est éparpillée) :
  ///
  /// ```txt
  ///       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮           ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
  /// ┌─────▼─────┬───────────┬─────o─────┬─────┴─────┬───────────┬─────▼─────┐
  /// │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #4 │ Sector #N │
  /// │   FAT #1  │  Content  │   FAT #0  │   FAT #2  │  Content  │    etc.   │
  /// └─────┬─────┴───────────┴───────────┴─────▲─────┴───────────┴─────┬─────┘
  ///       ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯                       ╰╍╍╍ ...
  /// ```
  ///
  /// C'est alors la DIFAT qui déclare le chaînage des secteurs FAT. L'exemple
  /// ci-dessus donnera la DIFAT suivante :
  ///
  /// | Indice DIFAT | Secteur FAT |
  /// |:------------:|:-----------:|
  /// |      #0      |     #2      |
  /// |      #1      |     #0      |
  /// |      #2      |     #3      |
  /// |      #3      |     #N      |
  /// |      #N      |     #...    |
  #[inline(always)]
  pub fn locate_fat_id(&self, sector_id: CfSectorId) -> CfResult<CfFatId> {
    // On divise par le nombre d'entrées par FAT pour avoir la n-ième FAT.
    self.nth_fat((sector_id as usize) / self.version.entries_per_fat())
  }

  /// Recherche l'[identifiant][super::CfFatId] de la n-ième FAT dans le
  /// fichier.
  ///
  /// Si la FAT demandée fait partie des 109 premières alors la DIFAT de
  /// l'en-tête suffit pour la recherche. Autrement, les DIFAT sont simplement
  /// chaînées entre elles ("les DIFAT" = abus de langage pour dire les secteurs
  /// dédiés aux DIFAT). Dans ce cas, l'[en-tête][super::CfHeader] pointe vers
  /// la première DIFAT puis chaque DIFAT pointe vers la prochaine.
  ///
  /// C'est la dernière entrée de la DIFAT qui est réservée pour pointer vers la
  /// prochaine. La valeur magique `ENDOFCHAIN` (`0xFFFFFFFE`) indique la fin de
  /// la chaîne. En version 3 il y a 127 entrées par DIFAT et 1023 en version 4.
  ///
  /// ```txt
  ///       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
  ///       │                       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮    │
  /// ┌─────o─────┬───────────┬─────▼─────┬───────────┬───┴────▼──┬───────────┐
  /// │ CF Header │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #N │
  /// │ 109 DIFAT │  FAT #300 │  DIFAT #1 │   FAT #3  │  DIFAT #0 │    etc.   │
  /// └─────o─────┴─────▲─────┴─────┬─────┴─────▲─────┴───────────┴───────────┘
  ///       │           ╰╍╍╍╍╍╍╍╍╍╍╍╯           │                   512 bytes
  ///       ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
  /// ```
  ///
  /// Dans l'exemple ci-dessus, la FAT `#3` est directement accessible depuis la
  /// DIFAT du header (`3 < 109`) alors que la localisation de la FAT `#300` se
  /// trouve dans le DIFAT `#1` (`109 + 127 < #300 < 109 + 127 + 127` soit
  /// `236 < #300 < 363`).
  pub fn nth_fat(&self, nth_fat: usize) -> CfResult<CfFatId> {
    let difat = unsafe { self.difat_header() }?;
    match difat.get(nth_fat) {
      Some(sector_id) => Ok(*sector_id),

      // Les DIFAT sont simplement chaînées entre elles.
      None => bail!(
        CfErrorInfo::Todo,
        "Pour l'instant, la \"première\" DIFAT suffit."
      ),
    }
  }

  /// Retourne le prochain [numéro de secteur][super::CfSectorId] de la chaîne
  /// de secteur.
  ///
  /// Une chaîne de secteur peut être en partie contiguë dans le fichier et/ou
  /// être complètement éparpillée dans celui-ci. La FAT, pour un
  /// [numéro de secteur][super::CfSectorId], donnera le prochain numéro de la
  /// chaîne de secteur qu'il constitue.
  ///
  /// ```txt
  ///       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮   ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮   ╭╍ ...
  /// ┌─────o─────┬───────────┬───────────┬───▼───┴───┬───────────┬───▼───┴───┐
  /// │ Sector #0 │ Sector #1 │ Sector #2 │ Sector #3 │ Sector #4 │ Sector #5 │
  /// │  Chain #1 │  Chain #0 │  Chain #0 │  Chain #1 │  Chain #0 │  Chain #1 │
  /// └───────────┴───┬───▲───┴─────o─────┴───────────┴───▲───┬───┴───────────┘
  ///                 │   ╰╍╍╍╍╍╍╍╍╍╯                     │   ╰╍╍╍╍╍╍╍╍╍╍╍╍╍ ...
  ///                 ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
  /// ```
  ///
  /// La première chaîne commence au secteur `#2` et la deuxième chaîne au
  /// secteur `#0`. Ces deux chaînes donneront alors dans la FAT :
  ///
  /// | Indice FAT | Prochain Secteur |
  /// |:----------:|:----------------:|
  /// |     #0     |        #3        |
  /// |     #1     |        #4        |
  /// |     #2     |        #1        |
  /// |     #3     |        #5        |
  /// |     #4     |        #...      |
  /// |     #5     |        #...      |
  pub fn next_sector_id(&self, sector_id: CfSectorId) -> CfResult<CfSectorId> {
    let fat_id = self.locate_fat_id(sector_id)?;
    let fat = unsafe { self.locate_sector_as::<u32>(fat_id) }?;

    debug_assert!(fat.len() == self.version.entries_per_fat());
    match fat.get((sector_id as usize) % self.version.entries_per_fat()) {
      Some(next_sector_id) => Ok(*next_sector_id),
      None => bail!(CfErrorInfo::SectorNotFound(sector_id)),
    }
  }
}

impl<R: CfReader> CfAnyCore for CfCore<R> {
  fn sector_size(&self) -> usize {
    self.version.sector_size()
  }

  fn next_sector_id(&self, sector_id: CfSectorId) -> CfResult<CfSectorId> {
    Self::next_sector_id(self, sector_id)
  }

  fn locate_sector(&self, sector_id: CfSectorId) -> CfResult<&[u8]> {
    Self::locate_sector(self, sector_id)
  }
}
