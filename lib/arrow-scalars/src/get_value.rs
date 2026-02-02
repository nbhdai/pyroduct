use crate::{ArrowRow, ArrowValue, PrimitiveValueList};
use half::f16;

// Macro to generate scalar accessors (e.g., get_bool, get_deep_bool)
macro_rules! impl_scalar_accessor {
    ($type_name:ident, $rust_type:ty, $variant:ident) => {
        paste::paste! {
            pub fn [<get_ $type_name>](&self, key: &str) -> Option<$rust_type> {
                match self.get(key) {
                    Some(ArrowValue::$variant(v)) => Some(*v),
                    _ => None,
                }
            }

            pub fn [<get_deep_ $type_name>]<S: AsRef<str>>(&self, path: &[S]) -> Option<$rust_type> {
                match self.get_deep(path) {
                    Some(ArrowValue::$variant(v)) => Some(*v),
                    _ => None,
                }
            }
        }
    };
}

// Macro to generate slice accessors for PrimitiveValueList (e.g., get_bool_slice)
macro_rules! impl_slice_accessor {
    ($type_name:ident, $rust_type:ty, $variant:ident) => {
        paste::paste! {
            pub fn [<get_ $type_name _slice>](&self, key: &str) -> Option<&[$rust_type]> {
                match self.get(key) {
                    Some(ArrowValue::PrimitiveList(PrimitiveValueList::$variant(cow))) => Some(cow.as_ref()),
                    _ => None,
                }
            }

            pub fn [<get_deep_ $type_name _slice>]<S: AsRef<str>>(&self, path: &[S]) -> Option<&[$rust_type]> {
                match self.get_deep(path) {
                    Some(ArrowValue::PrimitiveList(PrimitiveValueList::$variant(cow))) => Some(cow.as_ref()),
                    _ => None,
                }
            }
        }
    };
}

impl<'a> ArrowRow<'a> {
    // -------------------------------------------------------------------------
    // Boolean Accessors
    // -------------------------------------------------------------------------
    impl_scalar_accessor!(bool, bool, Bool);
    impl_slice_accessor!(bool, bool, Bool);

    // -------------------------------------------------------------------------
    // Unsigned Integer Accessors
    // -------------------------------------------------------------------------
    impl_scalar_accessor!(u8, u8, U8);
    impl_scalar_accessor!(u16, u16, U16);
    impl_scalar_accessor!(u32, u32, U32);
    impl_scalar_accessor!(u64, u64, U64);

    impl_slice_accessor!(u8, u8, U8);
    impl_slice_accessor!(u16, u16, U16);
    impl_slice_accessor!(u32, u32, U32);
    impl_slice_accessor!(u64, u64, U64);

    // -------------------------------------------------------------------------
    // Signed Integer Accessors
    // -------------------------------------------------------------------------
    impl_scalar_accessor!(i8, i8, I8);
    impl_scalar_accessor!(i16, i16, I16);
    impl_scalar_accessor!(i32, i32, I32);
    impl_scalar_accessor!(i64, i64, I64);

    impl_slice_accessor!(i8, i8, I8);
    impl_slice_accessor!(i16, i16, I16);
    impl_slice_accessor!(i32, i32, I32);
    impl_slice_accessor!(i64, i64, I64);

    // -------------------------------------------------------------------------
    // Float Accessors
    // -------------------------------------------------------------------------
    impl_scalar_accessor!(f16, f16, F16);
    impl_scalar_accessor!(f32, f32, F32);
    impl_scalar_accessor!(f64, f64, F64);

    impl_slice_accessor!(f16, f16, F16);
    impl_slice_accessor!(f32, f32, F32);
    impl_slice_accessor!(f64, f64, F64);

    // -------------------------------------------------------------------------
    // String Accessors
    // -------------------------------------------------------------------------

    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(ArrowValue::Str(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    pub fn get_deep_str<S: AsRef<str>>(&self, path: &[S]) -> Option<&str> {
        match self.get_deep(path) {
            Some(ArrowValue::Str(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Interval Accessors
    // -------------------------------------------------------------------------

    /// Returns the days and milliseconds of an IntervalDayTime value.
    pub fn get_interval_day_time(&self, key: &str) -> Option<(i32, i32)> {
        match self.get(key) {
            Some(ArrowValue::IntervalDayTime { days, milliseconds }) => {
                Some((*days, *milliseconds))
            }
            _ => None,
        }
    }

    pub fn get_deep_interval_day_time<S: AsRef<str>>(&self, path: &[S]) -> Option<(i32, i32)> {
        match self.get_deep(path) {
            Some(ArrowValue::IntervalDayTime { days, milliseconds }) => {
                Some((*days, *milliseconds))
            }
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Utility Accessors
    // -------------------------------------------------------------------------

    /// Returns true if the key exists and the value is explicitly Null.
    /// Returns false if the key does not exist or the value is not Null.
    pub fn get_is_null(&self, key: &str) -> bool {
        match self.get(key) {
            Some(ArrowValue::Null) => true,
            _ => false,
        }
    }

    pub fn get_deep_is_null<S: AsRef<str>>(&self, path: &[S]) -> bool {
        match self.get_deep(path) {
            Some(ArrowValue::Null) => true,
            _ => false,
        }
    }
}
