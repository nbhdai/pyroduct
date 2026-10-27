use super::Typeable;
use chrono::{DateTime, Utc};
use half::f16;
use std::collections::{BTreeMap, HashMap};

use crate::schema::{Bow, PrimitiveDataType, PyroType};

macro_rules! impl_primitive {
    ($rust_type:ty, $pyro_variant:ident, $prim_variant:ident) => {
        impl Typeable for $rust_type {
            fn pyro_type() -> PyroType {
                PyroType::PrimitiveScalar(PrimitiveDataType::$pyro_variant)
            }

            fn primitive_data_type() -> Option<PrimitiveDataType> {
                Some(PrimitiveDataType::$prim_variant)
            }

            fn coercible() -> bool {
                true
            }
        }
    };
}

impl_primitive!(bool, Bool, Bool);
impl_primitive!(u8, U8, U8);
impl_primitive!(u16, U16, U16);
impl_primitive!(u32, U32, U32);
impl_primitive!(u64, U64, U64);
impl_primitive!(usize, U64, U64);
impl_primitive!(i8, I8, I8);
impl_primitive!(i16, I16, I16);
impl_primitive!(i32, I32, I32);
impl_primitive!(i64, I64, I64);
impl_primitive!(f16, F16, F16);
impl_primitive!(f32, F32, F32);
impl_primitive!(f64, F64, F64);

impl Typeable for String {
    fn pyro_type() -> PyroType {
        PyroType::Str
    }

    fn coercible() -> bool {
        true
    }
}

impl Typeable for &str {
    fn pyro_type() -> PyroType {
        PyroType::Str
    }

    fn coercible() -> bool {
        true
    }
}

impl Typeable for DateTime<Utc> {
    fn pyro_type() -> PyroType {
        PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp)
    }
    // coercible() -> false (default): Timestamp only supports identity repair
}

// =============================================================================
// Complex Type Implementations
// =============================================================================

/// Handles nullability.
/// `Option<T>` forwards the type of `T` but marks `is_nullable` as true.
impl<T: Typeable> Typeable for Option<T> {
    fn pyro_type() -> PyroType {
        T::pyro_type().to_nullable()
    }

    fn primitive_data_type() -> Option<PrimitiveDataType> {
        T::primitive_data_type()
    }

    fn is_nullable() -> bool {
        true
    }

    fn coercible() -> bool {
        T::coercible()
    }
}

/// Handles Lists.
/// Automatically selects `PrimitiveList` if `T` is a non-nullable primitive,
/// otherwise defaults to `List`.
impl<T: Typeable> Typeable for Vec<T> {
    fn pyro_type() -> PyroType {
        // If it is a primitive and NOT nullable (e.g. Vec<i32>), use optimized PrimitiveList.
        // If it is nullable (e.g. Vec<Option<i32>>), we must use the generic List.
        if let Some(prim) = T::primitive_data_type()
            && !T::is_nullable()
        {
            return PyroType::PrimitiveList(prim);
        }

        PyroType::List(T::pyro_type().into())
    }

    fn coercible() -> bool {
        T::coercible()
    }
}

/// Handles Arrays.
/// Maps to `PrimitiveFixedList` if `T` is a non-nullable primitive.
/// Maps to `List` (generic) otherwise, as `PyroType` does not support fixed lists of complex types.
impl<T: Typeable, const N: usize> Typeable for [T; N] {
    fn pyro_type() -> PyroType {
        if let Some(prim) = T::primitive_data_type()
            && !T::is_nullable()
        {
            return PyroType::PrimitiveFixedList(prim, N);
        }

        // Fallback for complex types (e.g. [String; 4])
        PyroType::List(T::pyro_type().into())
    }

    fn coercible() -> bool {
        T::coercible()
    }
}

/// Handles Maps.
impl<K: Typeable, V: Typeable> Typeable for HashMap<K, V> {
    fn pyro_type() -> PyroType {
        PyroType::Map {
            key: Bow::new(K::pyro_type()),
            value: Bow::new(V::pyro_type()),
        }
    }

    fn coercible() -> bool {
        K::coercible() && V::coercible()
    }
}

impl<K: Typeable, V: Typeable> Typeable for BTreeMap<K, V> {
    fn pyro_type() -> PyroType {
        PyroType::Map {
            key: Bow::new(K::pyro_type()),
            value: Bow::new(V::pyro_type()),
        }
    }

    fn coercible() -> bool {
        K::coercible() && V::coercible()
    }
}

impl<K: Typeable, V: Typeable> Typeable for crate::Map<K, V> {
    fn pyro_type() -> PyroType {
        BTreeMap::<K, V>::pyro_type()
    }

    fn coercible() -> bool {
        BTreeMap::<K, V>::coercible()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives() {
        assert_eq!(
            i32::pyro_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert_eq!(
            f64::pyro_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64)
        );
        assert_eq!(
            bool::pyro_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::Bool)
        );
    }

    #[test]
    fn test_strings() {
        assert_eq!(String::pyro_type(), PyroType::Str);
        assert_eq!(<&str>::pyro_type(), PyroType::Str);
    }

    #[test]
    fn test_options() {
        assert_eq!(
            Option::<i32>::pyro_type(),
            PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32)
        );
        assert!(Option::<i32>::is_nullable());
        assert!(!i32::is_nullable());
    }

    #[test]
    fn test_primitive_list_optimization() {
        assert_eq!(
            Vec::<i32>::pyro_type(),
            PyroType::PrimitiveList(PrimitiveDataType::I32)
        );
    }

    #[test]
    fn test_complex_list() {
        assert_eq!(
            Vec::<String>::pyro_type(),
            PyroType::List(PyroType::Str.into())
        );
    }

    #[test]
    fn test_nullable_primitive_list() {
        assert_eq!(
            Vec::<Option<i32>>::pyro_type(),
            PyroType::List(PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32).into(),)
        );
    }

    #[test]
    fn test_fixed_size_array() {
        assert_eq!(
            <[u8; 16]>::pyro_type(),
            PyroType::PrimitiveFixedList(PrimitiveDataType::U8, 16)
        );
    }

    #[test]
    fn test_map() {
        assert_eq!(
            HashMap::<String, i32>::pyro_type(),
            PyroType::Map {
                key: Bow::new(PyroType::Str),
                value: Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32))
            }
        );
    }

    // =================================================================
    // Coercible tests
    // =================================================================

    #[test]
    fn test_coercible_primitives() {
        assert!(i32::coercible());
        assert!(f64::coercible());
        assert!(bool::coercible());
        assert!(u8::coercible());
    }

    #[test]
    fn test_coercible_strings() {
        assert!(String::coercible());
        assert!(<&str>::coercible());
    }

    #[test]
    fn test_not_coercible_timestamp() {
        assert!(!DateTime::<Utc>::coercible());
    }

    #[test]
    fn test_coercible_option_delegates() {
        assert!(Option::<i32>::coercible());
        assert!(!Option::<DateTime<Utc>>::coercible());
    }

    #[test]
    fn test_coercible_vec_delegates() {
        assert!(Vec::<i32>::coercible());
        assert!(Vec::<String>::coercible());
        assert!(!Vec::<DateTime<Utc>>::coercible());
    }

    #[test]
    fn test_coercible_array_delegates() {
        assert!(<[u8; 16]>::coercible());
        assert!(!<[DateTime<Utc>; 4]>::coercible());
    }

    #[test]
    fn test_coercible_map() {
        assert!(HashMap::<String, i32>::coercible());
        // Key is coercible but value is not
        assert!(!HashMap::<String, DateTime<Utc>>::coercible());
    }
}
