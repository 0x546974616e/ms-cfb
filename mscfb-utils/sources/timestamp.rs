use std::fmt;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
  // 1-based
  pub year: u32,
  pub month: u32,
  pub day: u32,

  // 0-based
  pub hour: u32,
  pub minute: u32,
  pub second: u32,
}

impl fmt::Debug for Timestamp {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    #[rustfmt::skip] // Sur une ligne c'est ok 👌.
    let Timestamp { year, month, day, hour, minute, second } = self;

    if year | month | day | hour | minute | second == 0 {
      return write!(formatter, "(zero)");
    }

    write!(
      formatter,
      "{year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    )
  }
}

impl fmt::Display for Timestamp {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    #[rustfmt::skip] // Sur une ligne c'est ok 👌.
    let Timestamp { year, month, day, hour, minute, second } = self;

    if year | month | day | hour | minute | second == 0 {
      return write!(formatter, "0000-??? ?? 00:00");
    }

    const MONTHS: [&str; 13] = [
      "???", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
      "Oct", "Nov", "Dec",
    ];

    write!(
      formatter,
      "{year:>4}-{month:02} {day:>2} {hour:02}:{minute:02}",
      month = MONTHS.get(*month as usize).unwrap_or(&"???"),
    )
  }
}

impl Timestamp {
  /// Crée un [`Timestamp`] depuis un [`FILETIME`][FILETIME] Windows.
  ///
  /// [FILETIME]: https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime
  ///
  /// La structure FILETIME est une valeur de 64 bits représentant le nombre
  /// d’intervalles de 100 nanosecondes écoulés depuis le 1er janvier 1601, en
  /// temps universel coordonné (UTC), soit `1601-01-01 00:00:00 UTC`. Windows
  /// stocke cette valeur en little-endian.
  ///
  /// Voir Windows Data Types `[MS-DTYP]`.
  pub fn from_filetime(bytes: &[u8; 8]) -> Self {
    let filetime = u64::from_le_bytes(*bytes);

    if filetime == 0x00 {
      // Tout à 0x00 indique que la date n'est pas valide ; qu'elle ne date
      // l'objet qu'elle réprésente. Ça n'a alors pas de sens de la parser.
      return Timestamp::default();
    }

    // Un FILETIME est exprimé en intervalles de 100 nanosecondes. Il y a
    // 10_000_000 intervalles de 100 nanosecondes dans une seconde. La division
    // entière permet donc d'obtenir le nombre de secondes complètes écoulées
    // depuis le `1601-01-01`.
    let total_seconds = filetime / 10_000_000;

    // On récupère les secondes dans la minute courante.
    let second = (total_seconds % 60) as u32;

    // On récupère le nombre total de minutes écoulées.
    let total_minutes = total_seconds / 60;

    // On récupère les minutes dans l'heure courante.
    let minute = (total_minutes % 60) as u32;

    // On récupère le nombre total d'heures écoulées.
    let total_hours = total_minutes / 60;

    // On récupère l'heure dans la journée courante.
    let hour = (total_hours % 24) as u32;

    // On récupère le nombre total de jours écoulés.
    let total_days = total_hours / 24;

    // On a maintenant `hh-mm-ss`. Il reste à convertir le nombre de
    // jours depuis le `1601-01-01` en date du calendrier grégorien.
    let (year, month, day) = days_to_date(total_days);

    Timestamp {
      year,
      month,
      day,
      hour,
      minute,
      second,
    }
  }
}

/// Convertit un nombre de jours écoulés depuis le `1601-01-01`.
// NOTE: Implémentation naïve, est-ce qu'il y a plus direct ? Quitte à LUT ?
fn days_to_date(mut days: u64) -> (u32, u32, u32) {
  let mut year: u32 = 1601;
  let mut month: u32 = 1;

  loop {
    // On boucle tant que le nombre de jours ne tient pas dans une année.
    let days_in_year = if is_leap_year(year) { 366 } else { 365 };
    if days < days_in_year {
      break;
    }

    days -= days_in_year;
    year += 1;
  }

  let february = if is_leap_year(year) { 29 } else { 28 };
  let days_in_month = [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

  // On boucle tant que le nombre de jours ne tient pas dans le mois courant.
  for month_days in days_in_month {
    if days < month_days {
      break;
    }

    days -= month_days;
    month += 1;
  }

  let day = days as u32 + 1;
  (year, month, day)
}

/// Indique si une année est bissextile.
///
/// Une année est bissextile si :
/// - elle est divisible par 4 mais pas par 100,
/// - elle divisible par 400.
fn is_leap_year(year: u32) -> bool {
  (year.is_multiple_of(4) && !year.is_multiple_of(100))
    || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn from_filetime() {
    let bytes = [0x00, 0x9C, 0xD5, 0xC2, 0x03, 0xE4, 0xA5, 0x01];
    assert_eq!(
      Timestamp::from_filetime(&bytes),
      Timestamp {
        year: 1977,
        month: 4,
        day: 24,
        hour: 1,
        minute: 30,
        second: 0
      }
    );
  }

  #[test]
  fn timestamp_to_string() {
    assert_eq!(
      "2026-08-30T17:02:34Z",
      format!(
        "{:?}",
        Timestamp {
          year: 2026,
          month: 08,
          day: 30,
          hour: 17,
          minute: 02,
          second: 34,
        }
      ),
    );
  }
}
