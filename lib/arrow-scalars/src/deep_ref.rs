use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub trait DeepRef {
    // The associated type allows Foo to return FooRef, and Bar to return BarRef
    type Ref<'a>
    where
        Self: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a>;
}

// =========================================================================
// 1. Primitive Scalars (Identity Transformation)
// =========================================================================
// For primitives, the "Ref" is just a copy of the value itself.

macro_rules! impl_primitive_deep_ref {
    ($($t:ty),*) => {
        $(
            impl DeepRef for $t {
                type Ref<'a> = $t;
                fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
                    *self
                }
            }
        )*
    };
}

impl_primitive_deep_ref!(
    bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
);

// =========================================================================
// 2. Strings
// =========================================================================

impl DeepRef for String {
    type Ref<'a> = &'a str;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.as_str()
    }
}

impl DeepRef for str {
    type Ref<'a> = &'a str;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self
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
// 6. Rkyv Archived Implementations
// =========================================================================

use rkyv::boxed::ArchivedBox;
use rkyv::option::ArchivedOption;
use rkyv::string::ArchivedString;
use rkyv::vec::ArchivedVec;

// ArchivedString -> &str
impl DeepRef for ArchivedString {
    type Ref<'a> = &'a str;
    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.as_str()
    }
}

// ArchivedVec<Primitive> -> &[Primitive]
macro_rules! impl_rkyv_vec_primitive {
    ($($t:ty),*) => {
        $(
            impl DeepRef for ArchivedVec<$t> {
                type Ref<'a> = &'a [$t];
                fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
                    self.as_slice()
                }
            }
        )*
    };
}

impl_rkyv_vec_primitive!(
    bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
);

// ArchivedOption<T>
impl<TA> DeepRef for ArchivedOption<TA>
where
    TA: DeepRef,
{
    type Ref<'a>
        = Option<TA::Ref<'a>>
    where
        TA: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        self.as_ref().map(|inner| inner.as_deep_ref())
    }
}

// ArchivedBox<T>
impl<TA: DeepRef> DeepRef for ArchivedBox<TA> {
    type Ref<'a>
        = TA::Ref<'a>
    where
        TA: 'a;

    fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
        (**self).as_deep_ref()
    }
}
