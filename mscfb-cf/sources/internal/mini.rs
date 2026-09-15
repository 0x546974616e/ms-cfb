use super::{
  CfAnyCore, CfControlStream, CfCore, CfRc, CfReader, CfResult, CfSectorId,
};

/// Numéro d'un secteur dans le [mini stream][super::CfMiniStream].
///
/// Contrairement aux [numéros de secteurs][super::CfSectorId], le *mini stream*
/// ne contient pas d'en-tête ; le premier *mini secteur* est le premier du
/// stream. Les *mini secteurs* sont énumérés par leur ordre dans le *mini
/// stream* et commence au numéro ̀`#0`.
///
/// ```txt
/// ┌───────────────────────────────────────────────────────────────────┐
/// │          Mini Stream divided into 64 bytes-length sectors         │
/// ├────────────────┬────────────────┬────────────────┬────────────────┤
/// │ Mini Sector #0 │ Mini Sector #1 │ Mini Sector #2 │ Mini Sector #N │
/// └────────────────┴────────────────┴────────────────┴────────────────┘
/// ```
pub type CfMiniSectorId = u32;

/// [Chaîne de secteur interne][CfControlStream] contenant la mini FAT.
///
/// La mini FAT permet de localiser les chaînes de secteur dans le
/// [mini stream][CfMiniStream]. Elle est elle-même une
/// [chaîne de secteurs][super::CfAnySectorChain] (un stream) dans le
/// **Compound File**. La mini FAT et le mini stream forment à eux deux, en
/// quelque sorte, un **Compound File** au sein même d'un **Compound File**.
/// Pour comprendre les deux il faut comprendre le [premier][super::CfCore].
///
/// Vue logique de la mini FAT :
///
/// ```txt
/// ┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐
/// │ Mini FAT #0 │ Mini FAT #1 │ Mini FAT #2 │ Mini FAT #3 │ Mini FAT #N │
/// └─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘
/// ```
///
/// Vue dans le **Compound File** (la mini FAT est éparpillée) :
///
/// ```txt
///       ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮             ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
/// ┌─────▼───────┬─────────────┬─────o───────┬─────┴───────┬──────▼──────┐
/// │  Sector #0  │  Sector #1  │  Sector #2  │  Sector #3  │  Sector #N  │
/// │ Mini FAT #1 │   Content   │ Mini FAT #0 │ Mini FAT #2 │ Mini FAT #3 │
/// └─────┬───────┴─────────────┴─────────────┴─────▲───────┴──────┬──────┘
///       ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯              ╰╍╍╍ ...
/// ```
pub type CfMiniFat<R> = CfControlStream<u32, R>;

/// [Chaîne de secteur interne][super::CfControlStream] contenant le *mini
/// stream*.
///
/// Lorsqu'un *stream* (= une chaîne de secteurs) a une taille inférieure strict
/// à la taille limite des minis secteurs (ou *Mini Stream Cutoff Size* ), en
/// pratique **4096** octets), alors le contenu du *stream* doit être localisé
/// non pas dans les *stream* de "premier niveau" du **Compound File** mais dans
/// les *stream* de "second niveau", c'est-à-dire au sein du *mini stream*.
///
/// Le *mini stream* est composé de *mini secteur* de **64** octets. Il faut la
/// [mini FAT][CfMiniFat] pour les recomposer.
///
/// Vue des secteurs dans le **Compound File** (le *mini stream* peut être
/// éparpillé/désordonnée dans le fichier) :
///
/// ```txt
/// ┌──────────────────────────────────────────────────────────────────────┐
/// │            Compound File divided into equal-length sectors           │
/// ├────────────────┬─────────────────┬─────────────────┬─────────────────┤
/// │                │    Sector #0    │    Sector #1    │    Sector #2    │
/// │     Header     │  Mini Stream #1 │                 │  Mini Stream #0 │
/// │                ├──┬──┬──┬──┬──┬──┤  Something Else ├──┬──┬──┬──┬──┬──┤
/// └────────────────┴──┴──┴──┴──┴──┴──┴─────────────────┴──┴──┴──┴──┴──┴──┘
/// ```
///
/// Vue logique du *mini stream* (le nombre de *minis secteurs* dépend de la
/// taille du secteur, **8** en version 3 et **64** en version 4):
///
/// ```txt
/// ┌───────────────────────────┬───────────────────────────┬──────────╍╍╍
/// │       Mini Stream #0      │       Mini Stream #1      │      ...
/// ├──────┬──────┬──────┬──────┼──────┬──────┬──────┬──────┼──────┬───╍╍╍
/// │ MS#0 │ MS#1 │ MS#2 │ MS#3 │ MS#4 │ MS#5 │ MS#6 │ MS#7 │ MS#N │ ..
/// └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴───╍╍╍
///  (MS#X = Mini Sector #X)
/// ```
///
/// Et par dessus les *mini secteurs*, grâce à la [mini FAT][CfMiniFat], sont
/// [reconstruits][super::CfMiniCore] les *mini streams* :
///
/// ```txt
/// ┌──────────────────────────────────────────────────────────────────────┐
/// │           Mini Stream divided into 64 bytes-length sectors           │
/// ├──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬───────┤
/// │ MS#0 │ MS#1 │ MS#2 │ MS#3 │ MS#4 │ MS#5 │ MS#6 │ MS#7 │ MS#N │  ...  │
/// └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴───────┘
///               ┌──────┐             ┌──────┬──────┐
///               │ MS#2 ├╍╍╍╍╍╍╍╍╍╍╍╍╍▶ MS#5 │ MS#6 │               Chain #2
/// ┌──────┬──────┼──────┘      ┌──────┼──────┴──────┘      ┌──────┐
/// │ MS#0 │ MS#1 ├╍╍╍╍╍╍╍╍╍╍╍╍╍▶ MS#4 ├╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍▶ MS#N │ Chain #0
/// └──────┴──────┘      ┌──────┼──────┘             ┌──────┼──────┘
///                      │ MS#3 ◀╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍┤ MS#7 │        Chain #7
///                      └──────┘                    └──────┘
/// ```
pub type CfMiniStream<R> = CfControlStream<[u8; 64], R>;

/// Combine la [mini FAT][super::CfMiniFat] et le
/// [mini stream][super::CfMiniStream] pour pouvoir être utilisée de la même
/// façon que le [Compound File][super::CfCore] lui-même.
///
/// ```txt
///  Mini FAT (basically)
/// ┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐
/// │ Mini FAT #0 │ Mini FAT #1 │ Mini FAT #2 │ Mini FAT #4 │ Mini FAT #N │
/// └───▲─────┬───┴─────────────┴───▲─────┬───┴─────────────┴───▲─────┬───┘
///     ╰╍╍╍╮ ╰╍╍╍╍╍╍╍╍╍╍╍╮      ╭╍╍╯     ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮   ╰╍╍╮  ╰╍╍╍ ...
/// ┌───────o────────┬────▼──────┴────┬────────────────┬────▼──────┴────┐
/// │ Mini Sector #0 │ Mini Sector #1 │ Mini Sector #2 │ Mini Sector #N │
/// └────────────────┴────────────────┴────────────────┴────────────────┘
///  Mini Stream
/// ```
pub struct CfMiniCore<R: CfReader> {
  // Les deux pourraient être `Rc<>` s'il faut.
  mini_stream: CfMiniStream<R>,
  mini_fat: CfMiniFat<R>,
}

impl<R: CfReader> CfMiniCore<R> {
  #[inline(always)]
  pub fn core(&self) -> CfRc<CfCore<R>> {
    self.mini_stream.core()
  }

  pub fn new(mini_stream: CfMiniStream<R>, mini_fat: CfMiniFat<R>) -> Self {
    CfMiniCore {
      mini_stream,
      mini_fat,
    }
  }
}

impl<R: CfReader> CfAnyCore for CfMiniCore<R> {
  fn sector_size(&self) -> usize {
    // TODO
    64
  }

  /// Retourne le prochain [numéro de mini secteur][super::CfMiniSectorId] de la
  /// chaîne de [mini secteur][super::CfMiniSectorIdChain].
  ///
  /// Voir la méthode [`CfCore::next_sector_id()`] pour plus de détails sur les
  /// chaînes de secteurs et la FAT.
  fn next_sector_id(&self, sector_id: CfSectorId) -> CfResult<CfSectorId> {
    self.mini_fat.nth_element(sector_id as usize).copied()
  }

  /// Récupère un mini secteur à partir de son numéro dans le mini stream.
  fn locate_sector(&self, sector_id: CfSectorId) -> CfResult<&[u8]> {
    self
      .mini_stream
      .nth_element(sector_id as usize)
      .map(AsRef::as_ref)
  }
}
