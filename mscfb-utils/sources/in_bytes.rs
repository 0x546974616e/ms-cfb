use std::fmt;

/// Affiche le type sous-jacent avec l'unité `bytes`.
///
/// ```rust
/// # use mscfb_utils::InBytes;
/// assert_eq!("9999", format!("{}", InBytes(9999)));
/// assert_eq!("10.0K", format!("{}", InBytes(10000)));
/// ```
///
/// Utilisation en tant que membre :
///
/// ```rust
/// # use mscfb_utils::InBytes;
///
/// #[derive(Debug)]
/// struct Dada {
///   dada: InBytes<usize>,
/// }
///
/// assert_eq!(
///   "Dada { dada: 42 bytes }",
///   format!("{:?}", Dada { dada: 42.into() })
/// );
/// ```
///
/// Ou en dérivant soi-même [`Debug`] :
///
/// ```rust
/// # use std::fmt;
/// # use mscfb_utils::InBytes;
///
/// struct Fafa {
///   fafa: usize,
/// }
///
/// impl fmt::Debug for Fafa {
///   fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
///     formatter
///       .debug_struct("Fafa")
///       .field("fafa", &InBytes(self.fafa))
///       .finish()
///   }
/// }
///
/// assert_eq!(
///   "Fafa { fafa: 69 bytes }",
///   format!("{:?}", Fafa { fafa: 69 })
/// );
/// ```
pub struct InBytes<T>(pub T);

impl<T> From<T> for InBytes<T> {
  fn from(value: T) -> Self {
    InBytes(value)
  }
}

impl<T: fmt::Debug> fmt::Debug for InBytes<T> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&self.0, formatter)?;
    formatter.write_str(" bytes")
  }
}

impl fmt::Display for InBytes<u64> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.0 < 10_000 {
      // En-dessous de 10000 ça reste lisible.
      return self.0.fmt(formatter);
    }

    // Système international d'unités (pas le système binaire avec 1024).
    const UNITS: &[&str] = &["K", "M", "G", "T"];
    const SI_BASE: f64 = 1000.0;

    let mut unit = 0;
    let mut value = self.0 as f64 / SI_BASE;
    while value >= SI_BASE && unit < UNITS.len() - 1 {
      value /= SI_BASE;
      unit += 1;
    }

    let width = formatter.width().unwrap_or(0).saturating_sub(1);
    write!(formatter, "{value:>width$.1}{}", UNITS[unit])
  }
}
