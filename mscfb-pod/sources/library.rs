#![doc = include_str!("../README.md")]
use std::mem::{align_of, size_of};

mod error;
pub use error::{PodError, PodResult};
pub use mscfb_pod_derive::Pod;
pub(crate) mod derive;

/// TLDR: Un type POD est une séquence de bits sans magie.
///
/// [source_pod]: https://stackoverflow.com/questions/45634083/is-there-a-concept-of-pod-types-in-rust
/// [source_static]: https://doc.rust-lang.org/rust-by-example/scope/lifetime/static_lifetime.html#trait-bound
///
/// Un type POD (Plain Old Data) est constitué uniquement de types primitifs
/// (`u8`, `i32`, `i64`...) et d'agrégations de types POD (`struct`,
/// `union`...). Une structure POD contient uniquement des types POD comme
/// membres et ne possède ni constructeurs, ni destructeurs, ni fonctions
/// virtuelles.
///
/// Les *trait bounds* suivants sont là pour faire respecter, au mieux,
/// [l'idée d'un type POD en Rust][source_pod] :
///
/// - [`'static`][source_static] signifie que le type ne contient aucune
///   référence interne non statique (`&T` et `&mut T`). Les types qui sont
///   `'static` n'ont donc aucune restriction de durée de vie et seront
///   globalement ignorés par le vérificateur d'emprunts.
///
/// - Le trait [`Sized`] exige que la taille du type soit connue à la
///   compilation et qu'il puisse ainsi être stocké sur la pile.
///
/// - Le trait [`Copy`] permet de dupliquer des valeurs simplement en copiant
///   leurs bits (sans *move semantics*). Le trait `Copy` est donc implémenté
///   par les types qui n'ont pas de gestion complexe de la mémoire, par exemple
///   avec une allocation sur le tas (les pointeurs) ou des références mutables
///   partagées (`&mut T`, alors que `&T` implémente `Copy`).
///
/// - [`Send`] and [`Sync`] exigent que le type puisse être envoyé vers d'autres
///   threads et qu'il puisse être partagé entre les threads via une référence
///   immuable (`&T`). Ces traits ne sont pas implémentés si le type contient un
///   mécanisme spécial quelconque (*interior mutability*, références sans durée
///   de vie...).
///
/// # Safety
///
/// Ce trait et ses méthodes sont `unsafe` car les *trait bounds* mentionnés
/// ci-dessus ne garantissent pas complètement qu'un type les implémentant soit
/// effectivement POD. Dans ce cas-là, une implémentation incorrecte pourrait
/// provoquer un *undefined behavior*. C'est alors à l'appelant de valider ce
/// qu'il fait.
pub unsafe trait Pod: 'static + Sized + Copy + Send + Sync {
  /// Retourne une référence vers le POD issue du *slice* ainsi que la partie
  /// restante inutilisée du *slice* après le POD.
  ///
  /// Si le *slice* est plus petit que le POD (en octets) ou si le *slice* n'est
  /// pas correctement aligné par rapport au POD, alors la fonction retourne une
  /// erreur.
  ///
  /// # Safety
  ///
  /// Dériver le trait [`Pod`] ne garantit pas complètement que le type le soit
  /// effectivement. C'est alors à l'appelant de la confirmer avec du `unsafe`.
  /// De plus, la suite d'octets est simplement réinterprété, l'endianness n'est
  /// donc pas pris en compte ⚠️.
  ///
  /// ```rust
  /// # use mscfb_pod::Pod;
  /// #[repr(C)]
  /// #[derive(Pod, Copy, Clone)]
  /// # #[derive(Debug, PartialEq, Eq)]
  /// struct Dada([u8; 3], u16);
  /// # assert_eq!(std::mem::align_of::<Dada>(), 2);
  /// # assert_eq!(std::mem::size_of::<Dada>(), 6);
  ///
  /// let bytes = [0x01, 0x02, 0x03, 0x04, 0x13, 0x37, 0x42, 0x69];
  /// let (dada, remaining) = unsafe { Dada::from_bytes(&bytes).unwrap() };
  /// # // Pour les architectures little-endian pour la clarté de la doc.
  /// # let mut dada = dada.clone(); dada.1 = u16::from_be(dada.1);
  ///
  /// assert_eq!(dada, Dada([0x01, 0x02, 0x03], 0x1337));
  /// assert_eq!(remaining, &[0x42, 0x69]);
  /// ```
  unsafe fn from_bytes(bytes: &[u8]) -> PodResult<(&Self, &[u8])> {
    match bytes.split_at_checked(size_of::<Self>()) {
      None => Err(PodError::SizeOfMismatch {
        size_of: size_of::<Self>(),
        length: bytes.len(),
      }),

      Some((bytes, remaining)) => {
        let pointer = bytes.as_ptr();
        #[cfg(any(clippy, not(feature = "unaligned")))]
        // TODO (nightly): ptr.is_aligned_to() est encore unstable.
        if !(pointer as usize).is_multiple_of(align_of::<Self>()) {
          return Err(PodError::AlignOfMismatch {
            align_of: align_of::<Self>(),
            pointer: pointer as usize,
          });
        }

        let pod = unsafe { &*pointer.cast::<Self>() };
        Ok((pod, remaining))
      }
    }
  }

  unsafe fn from_bytes_as_slice(_bytes: &[u8]) -> PodResult<(&[Self], &[u8])> {
    todo!("Le faire puis ensuite l'appeler dans CfCore::lookup_sector_as::<T>");
    // unsafe { transmute::<[u8; 0x70], Header>(header_data) };
  }
}

macro_rules! impl_pod {
  ($($TYPE: ident),+) => {
    $(unsafe impl Pod for $TYPE {})+
  };
}

unsafe impl Pod for () {}
unsafe impl<T: Pod, const N: usize> Pod for [T; N] {}
impl_pod!(i8, i16, i32, i64, i128, isize);
impl_pod!(u8, u16, u32, u64, u128, usize);
impl_pod!(bool, f32, f64);
