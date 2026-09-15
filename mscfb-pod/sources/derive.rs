#[macro_export]
macro_rules! derive_pod {
  // StructStruct (non generic)
  // https://doc.rust-lang.org/reference/items/structs.html#structs
  (
    $(#$STRUCT_META:tt)*
    $STRUCT_VISIBILITY:vis struct $STRUCT_NAME:ident {
      $(
        $(#[$FIELD_META:meta])*
        $FIELD_VISIBILITY:vis $FIELD_NAME:ident: $FIELD_TYPE:ty
      ),*
      $(,)?
    }
  ) => {
    #[automatically_derived]
    unsafe impl $crate::Pod for $STRUCT_NAME
      where Self: 'static $(, $FIELD_TYPE: $crate::Pod)* {}
  };

  // TupleStruct (non generic)
  // https://doc.rust-lang.org/reference/items/structs.html#structs
  (
    $(#$STRUCT_META:tt)*
    $STRUCT_VISIBILITY:vis struct $STRUCT_NAME:ident $((
      $(
        $(#[$FIELD_META:meta])*
        $FIELD_VISIBILITY:vis $FIELD_TYPE:ty
      ),*
      $(,)?
    ))?;
  ) => {
    #[automatically_derived]
    unsafe impl $crate::Pod for $STRUCT_NAME
      where Self: 'static $($(, $FIELD_TYPE: $crate::Pod)*)? {}
  };

  // StructStruct and TupleStruct (with generics)
  // https://doc.rust-lang.org/reference/items/structs.html#structs
  (
    $(#$STRUCT_META:tt)*
    $STRUCT_VISIBILITY:vis struct $STRUCT_NAME:ident < $($STRUCT_TAIL:tt)*
  ) => {
    compile_error!(
      concat!(
        "Cannot implement `Pod` for type `",
         stringify!($STRUCT_NAME), "`: generics or lifetimes are not allowed."
      )
    );
  };

  // Enumeration
  // https://doc.rust-lang.org/reference/items/enumerations.html#enumerations
  (
    $(#$ENUM_META:tt)*
    $ENUM_VISIBILITY:vis enum $ENUM_NAME:ident $($ENUM_TAIL:tt)*
  ) => {
    compile_error!(
      concat!(
        "Cannot implement `Pod` for type `",
         stringify!($ENUM_NAME), "`: enums are not allowed."
      )
    );
  };

  // Union
  // https://doc.rust-lang.org/reference/items/unions.html#unions
  (
    $(#$UNION_META:tt)*
    $UNION_VISIBILITY:vis union $UNION_NAME:ident $($TAIL:tt)*
  ) => {
    compile_error!(
      concat!(
        "Cannot implement `Pod` for type `",
         stringify!($UNION_NAME), "`: unions are not allowed."
      )
    );
  };

  // Invalid cases.
  ($($tt:tt)*) => {
    compile_error!(
      "Deriving `Pod` is not supported for this type."
    );
  };
}
