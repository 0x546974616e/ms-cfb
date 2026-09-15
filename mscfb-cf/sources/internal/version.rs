#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CfVersion {
  V3,
  V4,
}

impl CfVersion {
  ///
  /// Retourne le **sector shift**.
  ///
  /// En théorie un **Compound File** permet n'importe quelle puissance de 2
  /// pour définir la taille d'une taille. Cependant, la spécification impose
  /// *shift* de **9** (= secteur de 512 octets) pour la version 3 et un *shift*
  /// de **12** (= secteur 4096 de octets) pour la version 4.
  pub const fn sector_shift(&self) -> u16 {
    match self {
      Self::V3 => 0x0009,
      Self::V4 => 0x000C,
    }
  }

  ///
  /// Retourne le **mini sector shift**
  ///
  /// En théorie un **Compound File** permet n'importe quelle valeur mais la
  /// spécification impose un *shift* de **6** (= mini secteur de 64 octets).
  pub const fn mini_sector_shift(&self) -> u16 {
    0x0006u16
  }

  ///
  /// Retourne le **sector size**.
  ///
  /// En théorie un **Compound File** permet n'importe quelle taille de secteur
  /// (voir le [`sector_shift()`][Self::sector_shift]). Mais en pratique, la
  /// spécification impose une taille de **512** octets en version 3 et **4096**
  /// en version 4.
  pub const fn sector_size(&self) -> usize {
    match self {
      Self::V3 => 512,
      Self::V4 => 4096,
    }
  }

  ///
  /// Retourne le **mini sector size**.
  ///
  /// En théorie un **Compound File** permet n'importe quelle taille de mini
  /// secteur (voir le [`mini_sector_size()`][Self::mini_sector_size]) mais la
  /// spécification impose une taille de **64** octets.
  pub const fn mini_sector_size(&self) -> usize {
    64
  }

  /// Retourne le nombre d'entrées par secteur FAT.
  ///
  /// Un entrée FAT est un entier de 32 bits et son nombre par secteur dépend de
  /// la version du fichier. En version 3 il y a **512 / 4 = 128** entrées et
  /// **4096 / 4 = 1024** en version 4.
  pub const fn entries_per_fat(&self) -> usize {
    match self {
      Self::V3 => 128,
      Self::V4 => 1024,
    }
  }
}
