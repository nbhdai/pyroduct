//! Bridgeable implementations for common standard library types.
//!
//! This module provides explicit implementations of the `Bridgeable` trait
//! for standard library types. Users should use the `#[bridgeable]` macro
//! for their own types.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

use crate::format::{Bridgeable, rkyv_8::Rkyv};
// --- Macro to reduce boilerplate ---

/// Internal macro for implementing Bridgeable on types that satisfy rkyv bounds.
macro_rules! impl_bridgeable {
    ($ty:ty) => {
        impl Bridgeable for $ty {
            type Format = Rkyv<$ty>;
        }
    };
}

/// Internal macro for implementing Bridgeable on generic types with one type parameter.
macro_rules! impl_bridgeable_generic1 {
    ($ty:ident < $T:ident > $(where $($bound:tt)+)?) => {
        impl<$T> Bridgeable for $ty<$T>
        where
            $T: rkyv::Archive,
            <$T as rkyv::Archive>::Archived: 'static,
            <$ty<$T> as rkyv::Archive>::Archived: 'static,
            for<'a> $T: rkyv::Serialize<
                rkyv::rancor::Strategy<
                    rkyv::ser::Serializer<
                        &'a mut crate::format::PyroVec,
                        rkyv::ser::allocator::ArenaHandle<'a>,
                        rkyv::ser::sharing::Share,
                    >,
                    rkyv::rancor::Error,
                >,
            >,
            for<'a> <$ty<$T> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
                rkyv::rancor::Strategy<
                    rkyv::validation::Validator<
                        rkyv::validation::archive::ArchiveValidator<'a>,
                        rkyv::validation::shared::SharedValidator,
                    >,
                    rkyv::rancor::Error,
                >,
            >,
            <$ty<$T> as rkyv::Archive>::Archived:
                rkyv::Deserialize<$ty<$T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
            $($($bound)+)?
        {
            type Format = Rkyv<$ty<$T>>;
        }
    };
}
// TODO make a multiversion system
// Version bytes 200-255 signal an extended header. version = 203 means "read 3 more bytes (padded to alignment) for version info."
// Want me to implement the Phase 1 nibble-packing in the common.rs now? It would change the impl_bridgeable_generic2 macro to:
/// Internal macro for implementing Bridgeable on generic types with two type parameters.
macro_rules! impl_bridgeable_generic2 {
    ($ty:ident < $K:ident, $V:ident > $(where $($bound:tt)+)?) => {

        impl<$K, $V> Bridgeable for $ty<$K, $V>
        where
            $K: rkyv::Archive,
            <$K as rkyv::Archive>::Archived: 'static,
            $V: rkyv::Archive,
            <$V as rkyv::Archive>::Archived: 'static,
            <$ty<$K, $V> as rkyv::Archive>::Archived: 'static,
            for<'a> $K: rkyv::Serialize<
                rkyv::rancor::Strategy<
                    rkyv::ser::Serializer<
                        &'a mut crate::format::PyroVec,
                        rkyv::ser::allocator::ArenaHandle<'a>,
                        rkyv::ser::sharing::Share,
                    >,
                    rkyv::rancor::Error,
                >,
            >,
            for<'a> $V: rkyv::Serialize<
                rkyv::rancor::Strategy<
                    rkyv::ser::Serializer<
                        &'a mut crate::format::PyroVec,
                        rkyv::ser::allocator::ArenaHandle<'a>,
                        rkyv::ser::sharing::Share,
                    >,
                    rkyv::rancor::Error,
                >,
            >,
            for<'a> <$ty<$K, $V> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
                rkyv::rancor::Strategy<
                    rkyv::validation::Validator<
                        rkyv::validation::archive::ArchiveValidator<'a>,
                        rkyv::validation::shared::SharedValidator,
                    >,
                    rkyv::rancor::Error,
                >,
            >,
            <$ty<$K, $V> as rkyv::Archive>::Archived:
                rkyv::Deserialize<$ty<$K, $V>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
            $($($bound)+)?
        {
            type Format = Rkyv<$ty<$K, $V>>;
        }
    };
}

// --- Unit Type ---

impl Bridgeable for () {
    type Format = Rkyv<()>;
}

// --- Primitive Types ---

impl_bridgeable!(bool);
impl_bridgeable!(i8);
impl_bridgeable!(i16);
impl_bridgeable!(i32);
impl_bridgeable!(i64);
impl_bridgeable!(i128);
impl_bridgeable!(isize);
impl_bridgeable!(u8);
impl_bridgeable!(u16);
impl_bridgeable!(u32);
impl_bridgeable!(u64);
impl_bridgeable!(u128);
impl_bridgeable!(usize);
impl_bridgeable!(f32);
impl_bridgeable!(f64);
impl_bridgeable!(char);
impl_bridgeable!(String);

// --- Box<str> ---

impl Bridgeable for Box<str> {
    type Format = Rkyv<Box<str>>;
}

// --- Vec<T> ---

impl_bridgeable_generic1!(Vec<T>);

// --- Box<[T]> ---

impl<T> Bridgeable for Box<[T]>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
    <Box<[T]> as rkyv::Archive>::Archived: 'static,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut crate::format::PyroVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <Box<[T]> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <Box<[T]> as rkyv::Archive>::Archived:
        rkyv::Deserialize<Box<[T]>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    type Format = Rkyv<Box<[T]>>;
}

// --- Option<T> ---

impl_bridgeable_generic1!(Option<T>);

// --- HashMap<K, V> ---

impl_bridgeable_generic2!(HashMap<K, V> where
    K: Hash + Eq,
    <K as rkyv::Archive>::Archived: Hash + Eq
);

// --- HashSet<T> ---

impl<T> Bridgeable for HashSet<T>
where
    T: rkyv::Archive + Hash + Eq,
    <T as rkyv::Archive>::Archived: 'static,
    <HashSet<T> as rkyv::Archive>::Archived: 'static,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut crate::format::PyroVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <HashSet<T> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <T as rkyv::Archive>::Archived: Hash + Eq,
    <HashSet<T> as rkyv::Archive>::Archived:
        rkyv::Deserialize<HashSet<T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    type Format = Rkyv<HashSet<T>>;
}

// --- BTreeMap<K, V> ---

impl_bridgeable_generic2!(BTreeMap<K, V> where
    K: Ord,
    <K as rkyv::Archive>::Archived: Ord
);

// --- BTreeSet<T> ---

impl<T> Bridgeable for BTreeSet<T>
where
    T: rkyv::Archive + Ord,
    <T as rkyv::Archive>::Archived: 'static,
    <BTreeSet<T> as rkyv::Archive>::Archived: 'static,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut crate::format::PyroVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <BTreeSet<T> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <T as rkyv::Archive>::Archived: Ord,
    <BTreeSet<T> as rkyv::Archive>::Archived:
        rkyv::Deserialize<BTreeSet<T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    type Format = Rkyv<BTreeSet<T>>;
}

// --- Tuples ---

macro_rules! impl_bridgeable_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<$($T),+> Bridgeable for ($($T,)+)
        where
            $(
                $T: rkyv::Archive,
                <$T as rkyv::Archive>::Archived: 'static,
                for<'a> $T: rkyv::Serialize<
                    rkyv::rancor::Strategy<
                        rkyv::ser::Serializer<
                            &'a mut crate::format::PyroVec,
                            rkyv::ser::allocator::ArenaHandle<'a>,
                            rkyv::ser::sharing::Share,
                        >,
                        rkyv::rancor::Error,
                    >,
                >,
            )+
            <($($T,)+) as rkyv::Archive>::Archived: 'static,
            for<'a> <($($T,)+) as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
                rkyv::rancor::Strategy<
                    rkyv::validation::Validator<
                        rkyv::validation::archive::ArchiveValidator<'a>,
                        rkyv::validation::shared::SharedValidator,
                    >,
                    rkyv::rancor::Error,
                >,
            >,
            <($($T,)+) as rkyv::Archive>::Archived:
                rkyv::Deserialize<($($T,)+), rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
        {
            type Format = Rkyv<($($T,)+)>;
        }
    };
}

impl_bridgeable_tuple!(A);
impl_bridgeable_tuple!(A, B);
impl_bridgeable_tuple!(A, B, C);
impl_bridgeable_tuple!(A, B, C, D);
impl_bridgeable_tuple!(A, B, C, D, E);
impl_bridgeable_tuple!(A, B, C, D, E, F);
impl_bridgeable_tuple!(A, B, C, D, E, F, G);
impl_bridgeable_tuple!(A, B, C, D, E, F, G, H);
impl_bridgeable_tuple!(A, B, C, D, E, F, G, H, I);
impl_bridgeable_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_bridgeable_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_bridgeable_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

// --- Arrays ---
impl<T, const N: usize> Bridgeable for [T; N]
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
    <[T; N] as rkyv::Archive>::Archived: 'static,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut crate::format::PyroVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <[T; N] as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <[T; N] as rkyv::Archive>::Archived:
        rkyv::Deserialize<[T; N], rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    type Format = Rkyv<[T; N]>;
}

#[cfg(test)]
mod tests {
    use crate::format::{HasReceiver, Receiver, header::PyroHeader};

    use super::*;

    #[tracing_test::traced_test]
    #[test]
    fn test_primitive_ship() {
        let val: u64 = 42;
        let vec = val.ship().unwrap();
        println!("{vec:?}");
        let raw_slice = vec.as_raw_slice();
        println!("{raw_slice:?}");
    }

    #[test]
    fn test_primitive_roundtrip() {
        let val: u64 = 42;
        // we can encode it into a type:
        let vec = val.ship().unwrap();
        println!("{vec:?}");
        let raw_slice = vec.as_raw_slice();
        println!("{raw_slice:?}");
        assert_eq!(vec.wire_format(), 1);
        let typed = u64::expose(vec.view()).unwrap();
        // We now have zero copy access to the data
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        // We now have owned access to the original type.
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_string_roundtrip() {
        let val = "Hello, World!".to_string();
        let vec = val.ship().unwrap();
        let typed = String::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_vec_roundtrip() {
        let val: Vec<u32> = vec![1, 2, 3, 4, 5];
        let vec = val.ship().unwrap();
        let typed = Vec::<u32>::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_hashmap_roundtrip() {
        let mut val: HashMap<String, i32> = HashMap::new();
        val.insert("a".to_string(), 1);
        val.insert("b".to_string(), 2);

        let vec = val.ship().unwrap();
        let typed = HashMap::<String, i32>::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_option_roundtrip() {
        let val: Option<String> = Some("test".to_string());
        let vec = val.ship().unwrap();
        let typed = Option::<String>::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(val, recovered);

        let none_val: Option<String> = None;
        let vec = none_val.ship().unwrap();
        let typed = Option::<String>::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(none_val, recovered);
    }

    #[test]
    fn test_tuple_roundtrip() {
        let val: (u32, String, bool) = (42, "hello".to_string(), true);
        let vec = val.ship().unwrap();
        let typed = <(u32, String, bool)>::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_array_roundtrip() {
        let val: [u8; 4] = [1, 2, 3, 4];
        let vec = val.ship().unwrap();
        let typed = <[u8; 4]>::expose(vec.view()).unwrap();
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).unwrap();
        assert_eq!(val, recovered);
    }
}
