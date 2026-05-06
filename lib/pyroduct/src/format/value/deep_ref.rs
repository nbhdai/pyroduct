use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use half::f16;

/// A trait for types that can be converted into a "deep reference".
/// This is used to create zero-copy views of data.
///
/// Makes small allocations for things like Vec<&str> for strings.
pub trait DeepRef {
    // The associated type allows Foo to return FooRef, and Bar to return BarRef
    type Ref<'a>
    where
        Self: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a>;
}

pub trait FromRef<S> {
    fn from_ref(val: &S) -> Self;
}

// =========================================================================
// Primitives (identity, since primitives are Copy)
// =========================================================================

macro_rules! impl_from_ref_primitive {
    ($($t:ty),*) => {
        $(
            impl FromRef<$t> for $t {
                fn from_ref(val: &$t) -> $t {
                    *val
                }
            }

            impl FromRef<&$t> for $t {
                fn from_ref(val: &&$t) -> $t {
                    **val
                }
            }
        )*
    };
}

impl_from_ref_primitive!(
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64,
    bool, char
);

// =========================================================================
// &str → String
// =========================================================================

impl FromRef<&str> for String {
    fn from_ref(val: &&str) -> String {
        (**val).to_owned()
    }
}

// =========================================================================
// &[T] → Vec<T> for primitive types
// =========================================================================

macro_rules! impl_from_ref_primitive_slice {
    ($($t:ty),*) => {
        $(
            impl FromRef<&[$t]> for Vec<$t> {
                fn from_ref(val: &&[$t]) -> Vec<$t> {
                    (**val).to_vec()
                }
            }
        )*
    };
}

impl_from_ref_primitive_slice!(
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f16, f32, f64,
    bool, char
);

// =========================================================================
// &String → String  (String doesn't impl DeepRef, so needs explicit impl)
// =========================================================================

impl FromRef<&String> for String {
    fn from_ref(val: &&String) -> String {
        (**val).clone()
    }
}

// Vec<T> from borrowed &Vec<T>
impl<T: Clone> FromRef<&Vec<T>> for Vec<T> {
    fn from_ref(val: &&Vec<T>) -> Vec<T> {
        (**val).clone()
    }
}



impl DeepRef for Vec<String> {
    type Ref<'a> = Vec<&'a str>;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.iter().map(|s| s.as_str()).collect()
    }
}

impl DeepRef for Vec<&String> {
    type Ref<'a>
        = Vec<&'a str>
    where
        Self: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.iter().map(|s| s.as_str()).collect()
    }
}


impl<T, S: FromRef<T>> FromRef<Option<T>> for Option<S> {
    fn from_ref(val: &Option<T>) -> Self {
        match val {
            Some(t) => Some(FromRef::from_ref(t)),
            None => None,
        }
    }
}

impl<T, S: FromRef<T>, E, U: FromRef<E>> FromRef<Result<T, E>> for Result<S, U> {
    fn from_ref(val: &Result<T, E>) -> Self {
        match val {
            Ok(t) => Ok(FromRef::from_ref(t)),
            Err(e) => Err(FromRef::from_ref(e)),
        }
    }
}

// =========================================================================
// 3. Primitive Vectors (Zero-Copy Optimization)
// =========================================================================
// We manually implement these to ensure Vec<u8> returns &[u8] (slice)
// rather than Vec<&u8> (allocation).

macro_rules! impl_vec_primitive_deep_ref {
    ($($t:ty),*) => {
        $(
            impl DeepRef for Vec<$t> {
                type Ref<'a> = &'a [$t];

                fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
                    self.as_slice()
                }
            }

            // Also implement for the slice itself, just in case
            impl DeepRef for [$t] {
                type Ref<'a> = &'a [$t];

                fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
                    self
                }
            }
        )*
    };
}

impl_vec_primitive_deep_ref!(
    bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
);

// =========================================================================
// 4. Collections and Tuples
// =========================================================================

impl<K: DeepRef, V: DeepRef> DeepRef for HashMap<K, V>
where
    for<'a> <K as DeepRef>::Ref<'a>: Eq + std::hash::Hash,
{
    type Ref<'a>
        = HashMap<K::Ref<'a>, V::Ref<'a>>
    where
        K: 'a,
        V: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.iter()
            .map(|(k, v)| (k.as_deep_ref(), v.as_deep_ref()))
            .collect()
    }
}

impl<K: DeepRef, V: DeepRef> DeepRef for (K, V) {
    type Ref<'a>
        = (K::Ref<'a>, V::Ref<'a>)
    where
        K: 'a,
        V: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (self.0.as_deep_ref(), self.1.as_deep_ref())
    }
}

// =========================================================================
// 5. Wrappers (Option, Box, Arc, Rc, Cow)
// =========================================================================

impl<T: DeepRef> DeepRef for Option<T> {
    type Ref<'a>
        = Option<T::Ref<'a>>
    where
        T: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.as_ref().map(|inner| inner.as_deep_ref())
    }
}

impl<T: DeepRef + ?Sized> DeepRef for Box<T> {
    type Ref<'a>
        = T::Ref<'a>
    where
        T: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (**self).as_deep_ref()
    }
}

impl<T: DeepRef + ?Sized> DeepRef for Arc<T> {
    type Ref<'a>
        = T::Ref<'a>
    where
        T: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (**self).as_deep_ref()
    }
}

impl<T: DeepRef> DeepRef for [T] {
    type Ref<'a>
        = Vec<T::Ref<'a>>
    where
        T: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.iter().map(|t| t.as_deep_ref()).collect()
    }
}

impl<T: DeepRef> DeepRef for Vec<T> {
    type Ref<'a>
        = Vec<T::Ref<'a>>
    where
        T: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (**self).as_deep_ref()
    }
}

impl<T: DeepRef + ?Sized> DeepRef for Rc<T> {
    type Ref<'a>
        = T::Ref<'a>
    where
        T: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (**self).as_deep_ref()
    }
}

impl<'c, T: DeepRef + ToOwned + ?Sized> DeepRef for Cow<'c, T> {
    type Ref<'a>
        = T::Ref<'a>
    where
        T: 'a,
        'c: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (**self).as_deep_ref()
    }
}

// =========================================================================
// 7. PyroValue / PyroRow / PrimitiveValueList  (Normal types)
// =========================================================================
//
// These impls allow PyroValue<'a> and PyroRow<'a> to participate in the
// DeepRef ecosystem. The Ref output re-borrows into PyroValue<'a> /
// PyroRow<'a> with a shorter (or equal) lifetime, achieving zero-copy
// where possible (primitives are copied, strings/slices are re-borrowed).

use super::{PrimitiveValueList, PyroRow, PyroValue, Time};

// --- Time ---
// Time is Copy (i128 wrapper), so Ref is just Time.
impl DeepRef for Time {
    type Ref<'a> = Time;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        *self
    }
}

// --- PrimitiveValueList ---
// Re-borrows the inner Cow slices so the output lifetime is tied to &self.
impl<'v> DeepRef for PrimitiveValueList<'v> {
    type Ref<'a>
        = PrimitiveValueList<'a>
    where
        'v: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        match self {
            PrimitiveValueList::Bool(c) => PrimitiveValueList::Bool(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::U8(c) => PrimitiveValueList::U8(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::U16(c) => PrimitiveValueList::U16(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::U32(c) => PrimitiveValueList::U32(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::U64(c) => PrimitiveValueList::U64(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::I8(c) => PrimitiveValueList::I8(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::I16(c) => PrimitiveValueList::I16(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::I32(c) => PrimitiveValueList::I32(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::I64(c) => PrimitiveValueList::I64(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::F16(c) => PrimitiveValueList::F16(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::F32(c) => PrimitiveValueList::F32(Cow::Borrowed(c.as_ref())),
            PrimitiveValueList::F64(c) => PrimitiveValueList::F64(Cow::Borrowed(c.as_ref())),
        }
    }
}

// --- PyroValue ---
// Produces a PyroValue<'a> that borrows all string/slice data from &'a self.
// Primitives are copied (they're small scalars); complex recursive types
// (List, Group, MapInternal) are re-borrowed element-wise.
impl<'v> DeepRef for PyroValue<'v> {
    type Ref<'a>
        = PyroValue<'a>
    where
        'v: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        match self {
            // Scalars: Copy
            PyroValue::Null => PyroValue::Null,
            PyroValue::Bool(v) => PyroValue::Bool(*v),
            PyroValue::I8(v) => PyroValue::I8(*v),
            PyroValue::I16(v) => PyroValue::I16(*v),
            PyroValue::I32(v) => PyroValue::I32(*v),
            PyroValue::I64(v) => PyroValue::I64(*v),
            PyroValue::U8(v) => PyroValue::U8(*v),
            PyroValue::U16(v) => PyroValue::U16(*v),
            PyroValue::U32(v) => PyroValue::U32(*v),
            PyroValue::U64(v) => PyroValue::U64(*v),
            PyroValue::F16(v) => PyroValue::F16(*v),
            PyroValue::F32(v) => PyroValue::F32(*v),
            PyroValue::F64(v) => PyroValue::F64(*v),
            PyroValue::Timestamp(t) => PyroValue::Timestamp(*t),

            // String: re-borrow the Cow's inner &str
            PyroValue::Str(cow) => PyroValue::Str(Cow::Borrowed(cow.as_ref())),

            // PrimitiveList: re-borrow inner slices
            PyroValue::PrimitiveList(pl) => PyroValue::PrimitiveList(pl.as_deep_ref()),

            // Group (PyroRow): re-borrow
            PyroValue::Group(row) => PyroValue::Group(row.as_deep_ref()),

            // List: recursively re-borrow each element
            PyroValue::List(items) => {
                PyroValue::List(items.iter().map(|v| v.as_deep_ref()).collect())
            }

            // MapInternal: recursively re-borrow key-value pairs
            PyroValue::MapInternal(pairs) => PyroValue::MapInternal(
                pairs
                    .iter()
                    .map(|(k, v)| (k.as_deep_ref(), v.as_deep_ref()))
                    .collect(),
            ),
        }
    }
}

// --- PyroRow ---
// A PyroRow is a Vec<RowItem> where RowItem { key: Cow<str>, value: PyroValue }.
// We re-borrow both the key strings and the values.
impl<'v> DeepRef for PyroRow<'v> {
    type Ref<'a>
        = PyroRow<'a>
    where
        'v: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        // PyroRow already has a to_ref() method that does exactly this.
        // We delegate to it for consistency.
        self.clone()
    }
}
