use std::char::DecodeUtf16Error;
use std::cmp::Ordering;

/// Convertis une suite d'octets en chaîne de caractères Unicode.
///
/// ```rust
/// # use mscfb_utils::decode_utf16le;
///
/// let bytes = [
///   0x3D, 0xD8, 0x00, 0xDE, // 😀
///   0x53, 0x00, // S
///   0x6D, 0x00, // m
///   0x69, 0x00, // i
///   0x1E, 0xDD, // Invalid
///   0x6C, 0x00, // l
///   0x65, 0x00, // e
///   0x79, 0x00, // y
///   0x34, 0xD8, // Invaid
///   0x42, // Remaining
/// ];
///
/// let (iterator, remaining) = decode_utf16le(&bytes);
/// assert_eq!("😀Smi�ley�", iterator.collect::<String>());
/// assert_eq!(&[0x42], remaining);
/// ```
pub fn decode_utf16le(bytes: &[u8]) -> (impl Iterator<Item = char>, &[u8]) {
  let (chunks, remaining) = bytes.as_chunks::<2>();
  let code_units = chunks.iter().map(|w| u16::from_le_bytes(*w));
  let code_points = char::decode_utf16(code_units);
  (
    code_points.map(|x| x.unwrap_or(char::REPLACEMENT_CHARACTER)),
    remaining,
  )
}

/// Encode une chaîne UTF-8 en UTF-16LE.
///
/// Le caractère sentinelle n'est pas écrit (`&str` est un *slice*). Aucune
/// normalisation n'est effectuée.
///
/// ```rust
/// # use mscfb_utils::encode_utf16le;
///
/// let mut buffer = [0x00u8; 32];
///
/// assert_eq!(
///   encode_utf16le("Déjà😀", &mut buffer),
///   Some(
///     &[
///       0x44, 0x00, // D
///       0xE9, 0x00, // é
///       0x6A, 0x00, // j
///       0xE0, 0x00, // à
///       0x3D, 0xD8, 0x00, 0xDE // 😀
///     ][..]
///   )
/// );
/// ```
pub fn encode_utf16le<'a>(
  string: &str,
  buffer: &'a mut [u8],
) -> Option<&'a [u8]> {
  fn copy_byte((source, destination): (u8, &mut u8)) {
    *destination = source;
  }

  // On convertir la chaîne donnée en UTF-16LE.
  let mut encode = string.encode_utf16().flat_map(u16::to_le_bytes);
  let count = encode.by_ref().zip(&mut *buffer).map(copy_byte).count();

  // S'il en reste, c'est que la chaîne donnée est plus grande que le buffer.
  // Dans ce cas, on rend pas de chaîne tronquée.
  if encode.next().is_some() {
    return None;
  }

  // On récupère de la chaîne donnée seulement le nécessaire.
  buffer.get(..count)
}

/// Compare deux séquences d'octets comme de l'Utf16LE.
///
/// Compare les chaînes de caractères selon l’*algorithme de conversion de casse
/// par défaut* d'Unicode (**Unicode Default Case Conversion Algorithm**), la
/// *variante de conversion simple* (**Simple Case Foldings**), ou, en cas de
/// point de codes invalides, une comparaison binaire simple.
///
/// ⚠ Aucune normalisation effectuée.
pub fn compare_utf16le(a: &[u8], b: &[u8]) -> Ordering {
  let (chunks_a, remaining_a) = a.as_chunks::<2>();
  let (chunks_b, remaining_b) = b.as_chunks::<2>();

  let code_units_a = chunks_a.iter().copied().map(u16::from_le_bytes);
  let code_units_b = chunks_b.iter().copied().map(u16::from_le_bytes);

  let code_points_a = char::decode_utf16(code_units_a);
  let code_points_b = char::decode_utf16(code_units_b);

  let bytes_a = code_points_a.map(conversion);
  let bytes_b = code_points_b.map(conversion);

  // NOTE: Benchmark les performances, sinon faire main.
  fn conversion(result: Result<char, DecodeUtf16Error>) -> u32 {
    match result {
      Ok(character) => {
        if character.is_ascii() {
          // La plupart des chaînes devraient être en ASCII.
          return character.to_ascii_uppercase().into();
        }

        // Unicode Default Case Conversion.
        let mut characters = character.to_uppercase();

        // Simple Case Foldings implique la même taille.
        if characters.len() != 1 {
          return character.into();
        }

        let uppercased = characters.next();
        debug_assert!(characters.next().is_none());
        uppercased.unwrap_or(character).into()
      }

      // Sinon comparaison binaire.
      Err(error) => error.unpaired_surrogate().into(),
    }
  }

  match bytes_a.cmp(bytes_b) {
    Ordering::Equal => remaining_a.cmp(remaining_b),
    ordering => ordering,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn null_bytes() {
    let bytes = &[
      0x3D, 0xD8, 0x00, 0xDE, // 😀
      0x00, 0x00, // NULL
      0x3D, 0xD8, // Invalide
      0x41, 0x00, // A
      0x00, 0x00, // NULL
      0x41, 0x00, // A
      0x00, 0x00, // NULL
    ];

    assert_eq!(
      // On note alors que le caractère sentinelle ne termine pas le décodage.
      // C'est important à savoir pour la comparaison en fonction de l'origine
      // des chaînes testées.
      decode_utf16le(bytes).0.collect::<Vec<char>>(),
      ['😀', '\0', '�', 'A', '\0', 'A', '\0']
    );
  }

  #[test]
  fn encode_error() {
    let mut buffer = [0x00u8; 2];
    assert_eq!(encode_utf16le("AB", &mut buffer), None);
  }

  #[test]
  fn encode_string() {
    let mut buffer = [0x00u8; 64];
    assert_eq!(
      encode_utf16le("😀\0�A\0éÅÇ\01", &mut buffer),
      Some(
        &[
          0x3D, 0xD8, 0x00, 0xDE, // 😀
          0x00, 0x00, // \0
          0xFD, 0xFF, // �
          0x41, 0x00, // A
          0x00, 0x00, // \0
          0xE9, 0x00, // é
          0xC5, 0x00, // Å
          0xC7, 0x00, // Ç
          0x00, 0x00, // \0
          0x31, 0x00, // 1
        ][..]
      ),
    );
  }

  macro_rules! test_comparison {
    ( $( $NAME:ident($ORDERING:expr, $A:literal, $B:literal) )* ) => {
      $(
        #[test]
        fn $NAME() {
          let a = ($A).encode_utf16().flat_map(u16::to_le_bytes);
          let b = ($B).encode_utf16().flat_map(u16::to_le_bytes);
          assert_eq!(
            $ORDERING,
            compare_utf16le(
              &a.collect::<Vec<u8>>(),
              &b.collect::<Vec<u8>>(),
            ),
          );
        }
      )*
    };
  }

  test_comparison!(
    test_a_b(Ordering::Less, "A", "b")
    test_b_a(Ordering::Greater, "b", "a")
    test_b_aa(Ordering::Greater, "b", "aa")

    test_greek_a_b(Ordering::Less, "Α", "β")
    test_greek_b_a(Ordering::Greater, "β", "α")
    test_greek_b_aa(Ordering::Greater, "β", "αα")

    test_e_accent(Ordering::Equal, "é", "É")
    test_smiley(Ordering::Equal, "😀", "😀")

    // Pour démonter le Simple Case Folding.
    test_eszett(Ordering::Equal, "ß", "ß")
    test_eszett_simple_folding(Ordering::Greater, "ßß", "SS")

    test_omega(Ordering::Equal, "ω", "Ω")
    test_omaga_with_ascii(Ordering::Less, "ωa", "ΩB")

    test_misc_equal(
      Ordering::Equal,
      "öçôíòäáøðôþ3×æxvôähmdqÜL43ÁMÆÛÏEKZÅYWÚÓVDÙÄÀé1edaénåpuoèòkôócóççpð",
      "ÖÇÔÍÒÄÁØÐÔÞ3×ÆXVÔÄHMDQül43ámæûïekzåywúóvdùäàÉ1EDAÉNÅPUOÈÒKÔÓCÓÇÇPÐ"
    )

    test_misc_less(
      Ordering::Less,
      "öçôíòäáøðôþ3×æxvôähmdq=1",
      "ÖÇÔÍÒÄÁØÐÔÞ3×ÆXVÔÄHMDQ=2"
    )
  );

  #[test]
  fn test_decode_error() {
    let bytes = &[
      0x3D, 0xD8, 0x00, 0xDE, // 😀 U+1F600
      0x3D, 0xD8, // Invalide
      0x3D, 0xD8, // Invalide
      0x41, 0x00, // A
    ];

    let (chunks, remaining) = bytes.as_chunks::<2>();
    let code_units = chunks.iter().map(|w| u16::from_le_bytes(*w));
    let code_points = char::decode_utf16(code_units);
    let characters = code_points.map(|result| match result {
      Err(error) => error.unpaired_surrogate() as u32,
      Ok(character) => character as u32,
    });

    // Pour confirmer que lorsqu'il y a une erreur, on fera
    // bien une comparaison binaire sur le "mot" UTF-16.
    assert!(remaining.is_empty());
    assert_eq!(
      characters.collect::<Vec<u32>>(),
      [0x1F600, 0xD83D, 0xD83D, 0x41]
    );
  }
}
