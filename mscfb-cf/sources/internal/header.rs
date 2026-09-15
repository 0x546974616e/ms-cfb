use mscfb_pod::Pod;

use super::{CfErrorInfo, CfResult, CfVersion};
use crate::{bail, if_pedantic};

const CF_HEADER_SIGNATURE: [u8; 8] =
  [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// En-tête d'un **Compound File**.
#[repr(C)]
#[derive(Pod, Debug, Copy, Clone)]
pub struct CfHeader {
  header_signature: [u8; 8],
  header_clsid: [u8; 16],
  minor_version: u16,
  major_version: u16,
  byte_order: u16,
  sector_shift: u16,
  mini_sector_shift: u16,
  reserved: [u8; 6],
  number_dir_sector: u32,
  number_fat_sector: u32,
  pub first_directory_sector_id: u32,
  transaction_signature_number: u32,
  mini_stream_cutoff_size: u32,
  pub first_mini_fat_sector_id: u32,
  number_mini_fat_sector: u32,
  pub fitst_difat_sector_id: u32,
  number_difat_sector: u32,
}

impl CfHeader {
  /// Vérifie les champs de l'en-tête selon [MS-CFB] et retourne la version.
  ///
  /// [MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
  pub fn validate(&self) -> CfResult<CfVersion> {
    if self.header_signature != CF_HEADER_SIGNATURE {
      bail!(CfErrorInfo::BadHeaderSignature);
    }

    if_pedantic! {
      const CLSID_NULL: [u8; 16] = [0x00; 16];
      if self.header_clsid != CLSID_NULL {
        bail!(CfErrorInfo::BadHeaderClsid);
      }

      if self.minor_version != 0x003Eu16 {
        bail!(CfErrorInfo::BadMinorVersion);
      }
    }

    if !matches!(self.major_version, 3 | 4) {
      bail!(CfErrorInfo::BadMajorVersion);
    }

    if_pedantic! {
      if self.byte_order != 0xFFFEu16 {
        bail!(CfErrorInfo::BadByteOrder);
      }
    }

    let version =
      // La version a déjà été vérifiée.
      match self.major_version {
        3 => CfVersion::V3,
        _ => CfVersion::V4,
      };

    if self.sector_shift != version.sector_shift() {
      bail!(CfErrorInfo::BadSectorShift);
    }

    if_pedantic! {
      if self.mini_sector_shift != version.mini_sector_shift() {
        bail!(CfErrorInfo::BadMiniSectorShift);
      }

      const RESERVED_NULL: [u8; 6] = [0x00; 6];
      if self.reserved != RESERVED_NULL {
        bail!(CfErrorInfo::BadReservedField);
      }

      if self.major_version == 3 && self.number_dir_sector != 0 {
        bail!(CfErrorInfo::BadNumberOfDirectorySectors);
      }

      if self.mini_stream_cutoff_size != 0x00001000u32 {
        bail!(CfErrorInfo::BadMiniStreamCutoffSize);
      }
    }

    // La DIFAT est une structure à part. Elle a toujours 109 entrées de u32
    // quelle que soit la version. En version 3, la DIFAT termine le secteur.
    // En version 4, il reste 3584 octets qui DOIVENT être à zéro.
    {}

    Ok(version)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::mem::{align_of, size_of};

  #[test]
  fn size_and_align_of() {
    // Le header dans cette implémentation ne contient pas la DIFAT (c'est une
    // structure à part). Il y a 109 entrées indépendamment de SECTOR_SHIFT.
    let difat_size: usize = 109 * size_of::<u32>();
    assert_eq!(size_of::<CfHeader>(), 512 - difat_size);
    assert_eq!(align_of::<CfHeader>(), 4);
  }
}
