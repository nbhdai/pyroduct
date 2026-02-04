//! Bridgeable implementations for common standard library types.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

use crate::TypedBuf;
use crate::{BridgeVec, Bridgeable, PROTOCOL_VERSION, BridgeError};

// --- Primitive Types ---

impl Bridgeable for () {
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

impl Bridgeable for bool {
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

macro_rules! impl_bridgeable_primitive {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Bridgeable for $ty {
                fn serialize(&self) -> Result<BridgeVec, BridgeError> {
                    BridgeVec::serialize_from(self).map(|mut v| {
                        v.set_version(PROTOCOL_VERSION);
                        v
                    })
                }
                fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
                    vec.unchecked_parse::<Self>()
                }
                fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
                    vec.deserialize()
                }
            }
        )*
    };
}

impl_bridgeable_primitive!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, char,
);

// --- String Types ---

impl Bridgeable for String {
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

impl Bridgeable for Box<str> {
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- Vec ---

impl<T> Bridgeable for Vec<T>
where
    T: rkyv::Archive,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <Vec<T> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <Vec<T> as rkyv::Archive>::Archived:
        rkyv::Deserialize<Vec<T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- Box<[T]> ---

impl<T> Bridgeable for Box<[T]>
where
    T: rkyv::Archive,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
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
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- HashMap ---

impl<K, V> Bridgeable for HashMap<K, V>
where
    K: rkyv::Archive + Hash + Eq,
    V: rkyv::Archive,
    for<'a> K: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <K as rkyv::Archive>::Archived: Hash + PartialEq + Eq,
    for<'a> V: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <HashMap<K, V> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <HashMap<K, V> as rkyv::Archive>::Archived: rkyv::Deserialize<
            HashMap<K, V>,
            rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>,
        >,
{
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- HashSet ---

impl<T> Bridgeable for HashSet<T>
where
    T: rkyv::Archive + Hash + Eq,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
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
    <T as rkyv::Archive>::Archived: Hash + PartialEq + Eq,
    <HashSet<T> as rkyv::Archive>::Archived:
        rkyv::Deserialize<HashSet<T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- BTreeMap ---

impl<K, V> Bridgeable for BTreeMap<K, V>
where
    K: rkyv::Archive + Ord,
    V: rkyv::Archive,
    for<'a> K: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <K as rkyv::Archive>::Archived: Ord,
    for<'a> V: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <BTreeMap<K, V> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <BTreeMap<K, V> as rkyv::Archive>::Archived: rkyv::Deserialize<
            BTreeMap<K, V>,
            rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>,
        >,
{
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- BTreeSet ---

impl<T> Bridgeable for BTreeSet<T>
where
    T: rkyv::Archive + Ord,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <T as rkyv::Archive>::Archived: Ord,
    for<'a> <BTreeSet<T> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <BTreeSet<T> as rkyv::Archive>::Archived:
        rkyv::Deserialize<BTreeSet<T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- Option ---

impl<T> Bridgeable for Option<T>
where
    T: rkyv::Archive,
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
                    rkyv::ser::allocator::ArenaHandle<'a>,
                    rkyv::ser::sharing::Share,
                >,
                rkyv::rancor::Error,
            >,
        >,
    for<'a> <Option<T> as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'a>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
    <Option<T> as rkyv::Archive>::Archived:
        rkyv::Deserialize<Option<T>, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

// --- Tuples ---

macro_rules! impl_bridgeable_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<$($T),+> Bridgeable for ($($T,)+)
        where
            $(
                $T: rkyv::Archive,
                for<'a> $T: rkyv::Serialize<
                    rkyv::rancor::Strategy<
                        rkyv::ser::Serializer<&'a mut BridgeVec, rkyv::ser::allocator::ArenaHandle<'a>, rkyv::ser::sharing::Share>,
                        rkyv::rancor::Error
                    >
                >,
            )+
            for<'a> <($($T,)+) as rkyv::Archive>::Archived: rkyv::bytecheck::CheckBytes<
                rkyv::rancor::Strategy<
                    rkyv::validation::Validator<rkyv::validation::archive::ArchiveValidator<'a>, rkyv::validation::shared::SharedValidator>,
                    rkyv::rancor::Error
                >
            >,
            <($($T,)+) as rkyv::Archive>::Archived: rkyv::Deserialize<($($T,)+), rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
        {
            fn serialize(&self) -> Result<BridgeVec, BridgeError> {
                BridgeVec::serialize_from(self).map(|mut v| {
                    v.set_version(PROTOCOL_VERSION);
                    v
                })
            }
            fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
                vec.unchecked_parse::<Self>()
            }
            fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
                vec.deserialize()
            }
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
    for<'a> T: rkyv::Serialize<
            rkyv::rancor::Strategy<
                rkyv::ser::Serializer<
                    &'a mut BridgeVec,
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
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        BridgeVec::serialize_from(self).map(|mut v| {
            v.set_version(PROTOCOL_VERSION);
            v
        })
    }
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        vec.deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_roundtrip() {
        let val: u64 = 42;
        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = u64::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_string_roundtrip() {
        let val = "Hello, World!".to_string();
        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = String::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_vec_roundtrip() {
        let val: Vec<u32> = vec![1, 2, 3, 4, 5];
        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = Vec::<u32>::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_hashmap_roundtrip() {
        let mut val: HashMap<String, i32> = HashMap::new();
        val.insert("a".to_string(), 1);
        val.insert("b".to_string(), 2);

        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = HashMap::<String, i32>::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_option_roundtrip() {
        let val: Option<String> = Some("test".to_string());
        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = Option::<String>::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);

        let none_val: Option<String> = None;
        let vec = none_val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = Option::<String>::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(none_val, recovered);
    }

    #[test]
    fn test_tuple_roundtrip() {
        let val: (u32, String, bool) = (42, "hello".to_string(), true);
        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = <(u32, String, bool)>::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_array_roundtrip() {
        let val: [u8; 4] = [1, 2, 3, 4];
        let vec = val.serialize().unwrap();
        assert_eq!(vec.version(), PROTOCOL_VERSION);
        let typed = <[u8; 4]>::parse(vec).unwrap();
        let recovered = typed.deserialize().unwrap();
        assert_eq!(val, recovered);
    }
}