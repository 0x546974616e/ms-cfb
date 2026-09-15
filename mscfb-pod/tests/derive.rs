use mscfb_pod::Pod;

/// Vérifie que le `#[derive(Pod)]` fonctionne.
fn assert_pod<_T: Pod>() {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zst_struct() {
    #[derive(Pod, Copy, Clone)]
    struct Dada;
    assert_pod::<Dada>();
  }

  #[test]
  fn empty_struct() {
    #[derive(Pod, Copy, Clone)]
    pub struct Dada {}
    assert_pod::<Dada>();
  }

  #[test]
  fn empty_tuple_struct() {
    #[derive(Pod, Copy, Clone)]
    #[allow(clippy::needless_pub_self)]
    pub(self) struct Dada();
    assert_pod::<Dada>();
  }

  #[test]
  fn struct_attributes() {
    #[allow(unused)]
    #[derive(Pod, Copy, Clone)]
    pub(crate) struct Dada {
      a: [u8; 5],
      b: u128,
      c: u64,
      d: u32,
      e: u16,
      f: u8,
    }
    assert_pod::<Dada>();
  }

  #[test]
  fn tuple_struct_parameters() {
    #[allow(unused)]
    #[derive(Pod, Copy, Clone)]
    pub(in crate::tests) struct Dada([u8; 5], u128, u64, u32, u16, u8);
    assert_pod::<Dada>();
  }

  #[test]
  fn nested_struct() {
    #[allow(unused)]
    #[derive(Pod, Copy, Clone)]
    struct Dada([u8; 3], u16, u8);

    #[allow(unused)]
    #[derive(Pod, Copy, Clone)]
    struct Fafa {
      a: Dada,
      b: f32,
    }

    assert_pod::<Dada>();
  }
}
