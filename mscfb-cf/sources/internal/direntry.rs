use std::cmp::Ordering;
use std::fmt;

use mscfb_pod::Pod;
use mscfb_utils::{Timestamp, compare_utf16le, decode_utf16le, encode_utf16le};

use crate::{bail, if_pedantic};

use super::{
  CfControlStream, CfErrorInfo, CfObjectType, CfRc, CfReader, CfResult,
};

pub const CF_MAXREGSID: u32 = 0xFFFFFFFA;
pub const CF_NOSTREAM: u32 = 0xFFFFFFFF;

/// Un [nom][CfDirectoryName] de [répertoire][CfDirectoryEntry] ne peut pas
/// dépasser 64 octets et doit avoir une taille multiple de 2 (car UTF-16).
const CF_MAX_DIRECTORY_ENTRY_NAME: usize = 64;

/// Identifiant d'un [`CfDirectoryEntry`] au sein du
/// [Directory Stream][super::CfDirectoryStream].
pub type CfDirectoryId = u32;

/// Le **Directory** (ou **répertoire**) est la liste de toutes les [entrées du
/// répertoire][super::CfDirectoryEntry] pour les **Storage Object** et **Stream
/// Object**.
///
/// Une chaîne de secteur est réservée pour contenir toutes les [entrées du
/// répertoire][super::CfDirectoryEntry] du **Compound File**. Ces entrées ont
/// une taille fixe de **128** octets et un secteur en version 3 en contient
/// **4** alors qu'un secteur en version 4 en contient **32**.
///
/// L'identifiant d'une entrée correspond à l'indice de cette entrée dans le
/// tableau des entrées au-dessus de la chaîne associée. Donc pour récupérer une
/// entrée de répertoire il faut d'abord récupérer son secteur.
///
/// Vue logique des répertoires :
///
/// ```txt
/// ┌───────────────────┬───────────────────┬────────╍╍╍
/// │     Sector #2     │     Sector #0     │     ...
/// ├────┬────┬────┬────┼────┬────┬────┬────┼────┬───╍╍╍
/// │ D1 │ D2 │ D3 │ D4 │ D5 │ D6 │ D7 │ D8 │ D9 │ ..
/// └────┴────┴────┴────┴────┴────┴────┴────┴────┴───╍╍╍
///  (V3 = 512 bytes)
/// ```
///
/// Vue des répertoires dans le fichier :
///
/// ```txt
///           ╭╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╮
/// ┌─────────▼─────────┬───────────────────┬─────────o─────────┬────────╍╍╍
/// │     Sector #0     │     Sector #1     │     Sector #2     │     ...
/// ├────┬────┬────┬────┼───────────────────┼────┬────┬────┬────┼────┬───╍╍╍
/// │ D5 │ D6 │ D7 │ D8 │  Something Else   │ D1 │ D2 │ D3 │ D4 │ D9 │ ..
/// └────┴────┼────┴────┴───────────────────┴────┴────┴────┴────┴────┴─▲─╍╍╍
///           ╰╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╍╯
/// ```
pub type CfDirectoryStream<R> = CfControlStream<CfDirectoryEntry, R>;

impl<R: CfReader> CfDirectoryStream<R> {
  /// Cherche une entrée dans le répertoire donné.
  pub fn locate_entry(
    &self,
    current_id: CfDirectoryId,
    child_name: &str,
  ) -> CfResult<Option<&CfDirectoryEntry>> {
    // On récupère puis vérifie l'entrée.
    let entry = self.nth_element(current_id as usize)?;
    if entry.object_type()?.is_unallocated() {
      bail!(Todo, "is_unallocated");
    }

    // Et ensuite on compare (voir `try_compare()` pour plus de détails).
    match entry.name().try_compare(child_name)? {
      Ordering::Equal => return Ok(Some(entry)),
      Ordering::Less => {
        if entry.right_sibling != CF_NOSTREAM {
          return self.locate_entry(entry.right_sibling, child_name);
        }
      }
      Ordering::Greater => {
        if entry.left_sibling != CF_NOSTREAM {
          return self.locate_entry(entry.left_sibling, child_name);
        }
      }
    };

    Ok(None)
  }

  /// Parcours toutes les entrées du répertoire à partir de celui donné.
  /// C'est-à-dire tous ceux à *gauche* puis à *droite* récursivement (infix
  /// traversal) (mais par les *enfants* dans les répertoires sous-jacent).
  pub fn walk_directory(
    &self,
    directory_id: CfDirectoryId,
    callback: &mut impl FnMut(&CfDirectoryEntry) -> CfResult<()>,
  ) -> CfResult<()> {
    // Cette fonction augmente la stack mais c'est ok. De toute
    // façon il n'y a pas de détection de boucle (pas encore ?).
    let entry = self.nth_element(directory_id as usize)?;
    let object_type = entry.validate()?;

    if object_type.is_unallocated() {
      bail!(Todo, "is_unallocated");
    }

    if entry.left_sibling < CF_MAXREGSID {
      self.walk_directory(entry.left_sibling, callback);
    }

    let _: () = callback(&entry)?;

    if entry.right_sibling < CF_MAXREGSID {
      self.walk_directory(entry.right_sibling, callback);
    }

    Ok(())
  }
}

/// Entrée d'un répertoire.
#[repr(C)]
#[derive(Pod, Copy, Clone)]
pub struct CfDirectoryEntry {
  directory_name: [u8; CF_MAX_DIRECTORY_ENTRY_NAME],
  directory_name_length: u16,
  object_type: u8,
  color_flag: u8,
  pub left_sibling: u32,
  pub right_sibling: u32,
  pub child_id: u32,
  clsid: [u8; 16],
  state_bits: u32,
  creation_time: [u8; 8],
  modified_time: [u8; 8],
  pub starting_sector: u32,
  pub stream_size: u64,
}

impl CfDirectoryEntry {
  /// Voir [MS-OLEPS].
  #[inline(always)]
  pub fn is_property_set(&self) -> bool {
    self.directory_name.starts_with(&[0x05])
  }

  #[inline(always)]
  pub fn name(&self) -> CfDirectoryName<'_> {
    CfDirectoryName {
      name: &self.directory_name,
      length: self.directory_name_length as usize,
    }
  }

  #[inline(always)]
  pub fn object_type(&self) -> CfResult<CfObjectType> {
    CfObjectType::try_from(self.object_type)
  }

  /// Vérifie les champs du répertoire selon `MS-CFB` et retourne le
  /// [type du répertoire][CfObjectType].
  pub fn validate(&self) -> CfResult<CfObjectType> {
    if_pedantic! {
      if !self.directory_name_length.is_multiple_of(2) {
        bail!(CfErrorInfo::BadDirectoryNameLength);
      }

      if self.directory_name_length as usize > CF_MAX_DIRECTORY_ENTRY_NAME {
        bail!(CfErrorInfo::BadDirectoryNameLength);
      }

      let _: () = self.name().validate()?
    }

    let object_type = CfObjectType::try_from(self.object_type)?;

    if_pedantic! {
      if !matches!(self.color_flag, 0x0 | 0x1) {
        bail!(CfErrorInfo::BadColorFlag);
      }

      if object_type.is_stream() && self.child_id != CF_NOSTREAM {
        bail!(CfErrorInfo::BadChildId);
      }

      const CLSID_NULL: [u8; 16] = [0x00; 16];
      if object_type.is_stream() && self.clsid != CLSID_NULL {
        bail!(CfErrorInfo::BadDirectoryClsid);
      }
    }

    // NOTE: Techniquement, la date de création et la date de modification
    // doivent être à zéro pour les Stream Objects. Mais en pratique, je ne
    // crois pas que ce soit toujours le cas.
    {}

    if_pedantic! {
      if object_type.is_storage() && self.starting_sector != 0x00u32 {
        bail!(CfErrorInfo::BadStartingSectorLocation);
      }

      if object_type.is_storage() && self.stream_size != 0x00u64 {
        bail!(CfErrorInfo::BadStreamSize);
      }
    }

    Ok(object_type)
  }

  /// Change le nom du répertoire.
  pub fn set_name(&mut self, name: &str) -> CfResult<&Self> {
    match encode_utf16le(name, &mut self.directory_name) {
      None => bail!(DirectoryNameTooLong),
      Some(effective_name) => {
        self.directory_name_length = effective_name.len() as u16;
        Ok(self)
      }
    }
  }
}

impl fmt::Display for CfDirectoryEntry {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let modified_time = Timestamp::from_filetime(&self.modified_time);
    write!(
      formatter,
      "{letter}r--r--r-- {bytes:>6} {modified_time} {name}",
      bytes = mscfb_utils::InBytes(self.stream_size),
      letter = self.object_type().map_or('?', CfObjectType::letter),
      name = self.name()
    )
  }
}

/// Représente le nom d'un [`CfDirectoryEntry`].
pub struct CfDirectoryName<'directory> {
  name: &'directory [u8; CF_MAX_DIRECTORY_ENTRY_NAME],
  length: usize,
}

impl CfDirectoryName<'_> {
  /// Vérifie si le caractère est interdit pas la spécification.
  #[inline(always)]
  fn is_forbidden(character: &char) -> bool {
    matches!(character, '/' | '\\' | ':' | '!')
  }

  /// Retourne la longueur du nom (en octets, entre 0 et 64).
  ///
  /// Le caractères sentinelle n'est pas nécessairement inclus si le répertoire
  /// sous-jacent a déclaré une taille ne l'incluant pas (on n'y peut rien
  /// alors) ou si la taille déclarée est nulle et qu'on ne le trouve pas (dans
  /// ce cas on retourne la taille maximale).
  fn length_in_bytes(&self) -> usize {
    // J'ai constaté, en pratique (ou du moins avec les *Compound Files*
    // testés), et bien que la spécification n'en fasse aucune mention, que le
    // champ "Directory Name Length" est toujours à zéro au sein d'un fichier
    // même avec un "Directory Name" bien présent, valide et nécessaire. Dans ce
    // cas, on cherchera nous-mêmes la fin de la chaîne.
    if self.length > 0 {
      return self.length.min(CF_MAX_DIRECTORY_ENTRY_NAME);
    }

    // On cherche le premier u16 à 0x00 (le caractère sentinelle).
    // NOTE: On ne cherche pas l'optimisation, mais y'a des bit-hacks pour ça.
    let mut code_units = self.name.as_chunks::<2>().0.iter();
    let position = code_units.position(|bytes| *bytes == [0x00; 2]);

    // `*2` pour la taille de u16 à u8.
    // `+1` pour le caractère sentinelle inclus.
    let length = position.map(|x| x * 2 + 1);
    length.unwrap_or(CF_MAX_DIRECTORY_ENTRY_NAME)
  }

  /// Retourne le nom du répertoire (en octets) telle que le déclare sa
  /// [taille][Self::length_in_bytes] (sans nécessairement le caractère
  /// sentinelle si la chaîne n'a pas été [validée][Self::validate] avant).
  fn as_bytes(&self) -> &[u8] {
    self.name.get(..self.length_in_bytes()).unwrap_or(&[])
  }

  /// Retourne le nom du répertoire (en octets et
  /// sans caractère sentinelle s'il était présent).
  fn as_slice_nz(&self) -> &[u8] {
    let name = self.as_bytes();
    let length = name.len();

    let last_character = name.as_chunks::<2>().0.last();
    if last_character.map_or(false, |bytes| *bytes == [0x00; 2]) {
      // SAFETY: On soustrait donc le subslice existe nécessairement.
      unsafe { name.get_unchecked(..length.saturating_sub(2)) }
    } else {
      name
    }
  }

  // ⚠ Qu'est qui fait foi entre la taille de la chaîne et le caractère
  // sentinelle ? Est-ce que le premier lorsqu'elle est non nulle ou le second
  // quand elle est justement nulle ? Autrement, ça pue la "vulnérabilité" avec
  // des fichiers cachés dans le CF (dans l'idée).
  fn validate(&self) -> CfResult<()> {
    let name = self.as_bytes();

    // Un répertoire est au maximum une chaîne de 32 caractères Unicode
    // encodés en UTF-16LE (caractère sentinelle inclus et obligatoire).
    let last_character = name.as_chunks::<2>().0.last();
    if !last_character.map_or(false, |bytes| *bytes == [0x00; 2]) {
      bail!(CfErrorInfo::BadDirectoryName);
    }

    let (mut characters, remaining) = decode_utf16le(name);
    if !remaining.is_empty() {
      bail!(CfErrorInfo::BadDirectoryName);
    }

    // Les caractères '/', '\', ':' et '!' sont interdits.
    if characters.find(Self::is_forbidden).is_some() {
      bail!(CfErrorInfo::BadDirectoryName);
    }

    Ok(())
  }

  /// Comparaison entre deux noms de [répertoires][CfDirectoryEntry].
  ///
  /// La spécification (voir `MS-CFB`) impose la règle de comparaison suivante :
  /// un nom dont la taille est inférieure à celle d’un autre nom est considéré
  /// comme inférieur à celui-ci. La première étape de la comparaison se fait
  /// donc sur les *Directory Entry Name Length* (la taille en octets).
  ///
  /// Si deux noms ont la même taille, leurs points de code doivent ensuite être
  /// comparés un à un, dans l’ordre. Pour chaque point de code, il faut
  /// appliquer l’*algorithme de conversion de casse par défaut* d’Unicode
  /// ([Unicode Default Case Conversion Algorithm][DefaultCase]), la *variante
  /// de conversion simple* (simple case foldings). La comparaison se fait
  /// alors sur les valeurs binaires des points de code UTF-16 obtenus.
  ///
  /// La variante de conversion simple **ne change pas** la taille d'une chaîne.
  /// Ainsi, une conversion donnant une paire de points de code (les *surrogate
  /// pairs*) ou plus ne sera jamais conservée et le point de code orignal
  /// restera en minuscule.  Par exemple, le "eszett" allemand `ß` est comparé
  /// tel quel et non sous sa forme en majuscules `SS`.
  ///
  /// [UAX44]: https://www.unicode.org/reports/tr44/
  /// [DefaultCase]: https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-3/#G33992
  /// [Canonicalize]: https://tc39.es/ecma262/#sec-runtime-semantics-canonicalize-ch
  ///
  /// Liens pratiques : [Default Case][DefaultCase], [Unicode Character
  /// Database][UAX44] et la [spécification ECMA][Canonicalize]
  pub fn try_compare(&self, other: &str) -> CfResult<Ordering> {
    let mut buffer = [0x00u8; CF_MAX_DIRECTORY_ENTRY_NAME];
    let Some(other) = encode_utf16le(other, &mut buffer) else {
      bail!(CfErrorInfo::DirectoryNameTooLong);
    };

    // ⚠ La comparaison ne se fait caractère sentinelle.
    let name = self.as_slice_nz();
    let ordering = name.len().cmp(&other.len());
    if ordering != Ordering::Equal {
      return Ok(ordering);
    }

    // ⚠ Les chaînes ne sont pas normalisées.
    Ok(compare_utf16le(name, other))
  }
}

impl fmt::Display for CfDirectoryName<'_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let name = self.as_slice_nz();
    if name.is_empty() {
      return write!(formatter, "(empty)");
    }

    let (characters, remaining) = decode_utf16le(name);
    for character in characters {
      if character.is_ascii_control() {
        write!(formatter, "\\x{{{:02X}}}", character as u32)?;
      } else if character.is_control() {
        write!(formatter, "\\u{{{:04X}}}", character as u32)?;
      } else {
        write!(formatter, "{character}")?;
      }
    }

    for bytes in remaining {
      write!(formatter, "\\x{{{bytes:02X}}}")?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::mem::size_of;

  #[test]
  fn sizeo_of() {
    assert_eq!(128, size_of::<CfDirectoryEntry>());
  }
}
