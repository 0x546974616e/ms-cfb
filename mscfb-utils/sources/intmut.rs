//! Interior Mutability...

use std::cell::RefCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::RwLock;

macro_rules! impl_intmut {
  ($STRUCT:ident) => {
    impl<T> From<T> for $STRUCT<T> {
      fn from(value: T) -> Self {
        Self(value.into())
      }
    }

    impl<T> $STRUCT<T> {
      pub fn new(value: T) -> Self {
        Self::from(value)
      }

      // Parce que pénible de devoir importer le trait à chaque fois.
      // Le trait est surtout là s'il y a besoin de `impl InteriorMutability`.
      pub fn read(&self) -> impl Deref<Target = T> {
        InteriorMutability::read(&self.0)
      }

      // Idem.
      pub fn write(&self) -> impl DerefMut<Target = T> {
        InteriorMutability::write(&self.0)
      }
    }

    impl<T: fmt::Debug> fmt::Debug for $STRUCT<T> {
      fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, formatter)
      }
    }

    impl<T: fmt::Display> fmt::Display for $STRUCT<T> {
      fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.read().deref(), formatter)
      }
    }
  };
}

// Pas particulièrement fier de celui-là.
// NOTE: Peut-être retourner un `Result` à la place ?
pub trait InteriorMutability<T> {
  /// Récupère l'élément sous-jacent en lecture seule.
  ///
  /// # Panics
  ///
  /// La méthode `panic!()` si l'élément ne peut pas être récupéré.
  fn read(&self) -> impl Deref<Target = T>;

  /// Récupère l'élément sous-jacent en lecture et écriture.
  ///
  /// # Panics
  ///
  /// La méthode `panic!()` si l'élément ne peut pas être récupéré.
  fn write(&self) -> impl DerefMut<Target = T>;
}

impl<T> InteriorMutability<T> for RefCell<T> {
  fn read(&self) -> impl Deref<Target = T> {
    self.borrow()
  }

  fn write(&self) -> impl DerefMut<Target = T> {
    self.borrow_mut()
  }
}

impl<T> InteriorMutability<T> for RwLock<T> {
  fn read(&self) -> impl Deref<Target = T> {
    Self::read(self).unwrap()
  }

  fn write(&self) -> impl DerefMut<Target = T> {
    Self::write(self).unwrap()
  }
}

/// "RefCell" pour du monothread.
///
/// ```rust
/// # use std::rc::Rc;
/// # use mscfb_utils::SingleThreadedRefCell;
///
/// let value1 = Rc::new(SingleThreadedRefCell::new(42));
/// let value2 = value1.clone();
///
/// assert_eq!(42, *value1.read());
///
/// {
///   let mut n = value2.write();
///   *n += 1295;
/// }
///
/// assert_eq!(1337, *value1.read());
/// ```
#[repr(transparent)]
pub struct SingleThreadedRefCell<T>(RefCell<T>);

impl_intmut!(SingleThreadedRefCell);

/// "RefCell" pour du multithread.
///
/// ```rust
/// # use std::thread;
/// # use std::sync::Arc;
/// # use mscfb_utils::MultiThreadedRefCell;
/// # use mscfb_utils::InteriorMutability;
///
/// let value1 = Arc::new(MultiThreadedRefCell::new(420));
/// let value2 = value1.clone();
///
/// assert_eq!(420, *value1.read());
///
/// thread::spawn(move || {
///   let mut n = value2.write();
///   *n += 270;
/// })
/// .join()
/// .unwrap();
///
/// assert_eq!(690, *value1.read());
/// ```
#[repr(transparent)]
pub struct MultiThreadedRefCell<T>(RwLock<T>);

impl_intmut!(MultiThreadedRefCell);
