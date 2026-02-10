//! A Cow(Box) that allow for statics to be propagated with things like &str
//! 
//! For the eventual static macro that creates static schemas for structs.
//! 
//! It's hard to make the following work with the rest of this:

// #[derive(
//     Debug,
//     Clone,
//     PartialEq,
//     Eq,
//     Hash,
//     Serialize,
// )]
// pub enum StaticPyroType {
//     /// No value / unknown type (corresponds to `PyroValue::Null`).
//     Null,
//     /// Scalar primitive (Bool, Int, Float).
//     PrimitiveScalar(PrimitiveDataType),
//     /// UTF-8 string (corresponds to `PyroValue::Str`).
//     Str,
//     /// Day + millisecond interval (corresponds to `PyroValue::Timestamp`).
//     Timestamp,
//     /// Homogeneous list of a single primitive type (corresponds to `PyroValue::PrimitiveList`).
//     PrimitiveList(PrimitiveDataType),
//     /// Fixed-size homogeneous list of a single primitive type.
//     PrimitiveFixedList(PrimitiveDataType, usize),
//     /// Heterogeneous list of arbitrary pyro values (corresponds to `PyroValue::List`).
//     ///
//     /// Fields: `(element_type, element_nullable)`.
//     List(
//         &'static PyroType<'static>,
//         bool,
//     ),
//     /// Named struct / row (corresponds to `PyroValue::Group`).
//     Group(
//         &'static [PyroField<'static>],
//     ),
//     /// Key-value map (corresponds to `PyroValue::MapInternal`).
//     Map {
//         key: Box<PyroType<'static>>,
//         value: Box<PyroType<'static>>,
//     },
// }


use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A helper trait for defining the static owned counterpart of a type.
///
/// This is used by `Bow` to determine what type the `Static` variant holds.
/// For `str`, this is `str`. For `T: Sized`, this is `T`.
pub trait BowOwned {
    /// The resulting 'static type (usually Self).
    type Static: 'static + ?Sized;
    type Owned: Sized + 'static;
    fn to_owned(&self) -> Self::Owned;
}

pub type Bow<'a, T: BowOwned> = BowHolder<'a, T::Owned, T, T::Static>;

/// A Clone-on-Write smart pointer that is either a `Box<T::Owned>`, a `&T`, or a `&'static T::Static`.
///
/// This provides `Cow`-like semantics but guarantees that the owned variant
/// is a heap-allocated `Box<T>`.
/// 
/// This is for self referencing 
pub enum BowHolder<'a, B, T: ?Sized, S: 'static + ?Sized> {
    /// Owned data stored in a Box.
    Owned(Box<B>),
    /// Borrowed data.
    Borrowed(&'a T),
    /// Static reference (zero allocation, 'static lifetime).
    Static(&'static S),
}

impl<'a, B, T, S> BowHolder<'a, B, T, S> {
    /// Creates a Bow from a borrowed reference.
    pub const fn from_borrowed(shared: &'a T) -> Self {
        BowHolder::Borrowed(shared)
    }

    /// Creates a Bow from a static reference.
    pub const fn from_static(s: &'static S) -> Self {
        BowHolder::Static(s)
    }

    pub fn from_owned(val: B) -> Self {
        BowHolder::Owned(Box::new(val))
    }

    /// Returns true if the data is owned.
    pub fn is_owned(&self) -> bool {
        matches!(self, BowHolder::Owned(_))
    }

    /// Returns true if the data is borrowed.
    pub fn is_borrowed(&self) -> bool {
        matches!(self, BowHolder::Borrowed(_))
    }

    /// Returns true if the data is static.
    pub fn is_static(&self) -> bool {
        matches!(self, BowHolder::Static(_))
    }
}

impl<'a,T: BowOwned> Bow<'a, T> 
where Box<<T as BowOwned>::Owned>: Borrow<T>
{
    /// Converts this Bow into a 'static version of itself.
    ///
    /// Requires `T` to be `'static` (i.e., `T` cannot contain non-static references).
    pub fn to_static(self) -> BowHolder<'static, T::Owned, T, T::Static>
    where 
        T: BowOwned,
    {
        match self {
            BowHolder::Owned(b) => BowHolder::Owned(b),
            BowHolder::Static(s) => BowHolder::Static(s),
            BowHolder::Borrowed(b) => BowHolder::Owned(Box::from(b.to_owned())),
        }
    }
}

// --- Clone ---

impl<'a, B, T, S> Clone for BowHolder<'a, B, T, S>
where B: Clone 
{
    fn clone(&self) -> Self {
        match self {
            BowHolder::Borrowed(b) => BowHolder::Borrowed(b),
            BowHolder::Static(s) => BowHolder::Static(*s),
            BowHolder::Owned(o) => {
                BowHolder::Owned(o.clone())
            }
        }
    }
}

// --- Constructors ---

impl<'a, B, T, S> BowHolder<'a, B, T, S> {
    pub fn from_box(boxed: Box<B>) -> Self {
        BowHolder::Owned(boxed)
    }
}

// --- Standard Traits ---

impl<'a, B, T, S> Deref for BowHolder<'a, B, T, S>
where
    B: Deref<Target = T>,
    S: Borrow<T>,
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            BowHolder::Borrowed(borrowed) => borrowed,
            BowHolder::Owned(boxed) => boxed.deref(),
            BowHolder::Static(stat) => (*stat).borrow(),
        }
    }
}

impl<'a, B, T, S> Hash for BowHolder<'a, B, T, S>
where
    B: Hash,
    S: Hash + ?Sized,
    T: Hash + ?Sized,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BowHolder::Owned(b) => b.hash(state),
            BowHolder::Borrowed(t) => t.hash(state),
            BowHolder::Static(s) => s.hash(state),
        }
    }
}

// --- Serde: Serialize ---

impl<'a, B, T, S> Serialize for BowHolder<'a, B, T, S>
where
    B: Serialize,
    S: Serialize + ?Sized,
    T: Serialize + ?Sized,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        match self {
            BowHolder::Owned(b) => b.serialize(serializer),
            BowHolder::Borrowed(t) => t.serialize(serializer),
            BowHolder::Static(s) => s.serialize(serializer),
        }
    }
}

// --- Serde: Deserialize ---

impl<'de, 'a, B, T, S> Deserialize<'de> for BowHolder<'a, B, T, S>
where
    // We only need to be able to deserialize into the Owned type B.
    B: Deserialize<'de>,
    // T and S are phantom for the purpose of creating a fresh Owned instance.
    T: ?Sized,
    S: 'static + ?Sized,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Default behavior: Deserialize into B, then Box it.
        // This is safer than borrowing from the input for general cases, 
        // as it avoids complex lifetime dependencies on the Deserializer.
        B::deserialize(deserializer).map(|owned| BowHolder::Owned(Box::new(owned)))
    }
}

// --- Standard Traits: Deref (Refined) ---

// --- Debug Implementation ---
// Useful for ensuring derived Debug on structs using Bow works correctly
impl<'a, B, T, S> fmt::Debug for BowHolder<'a, B, T, S>
where
    B: fmt::Debug,
    S: fmt::Debug,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BowHolder::Owned(b) => b.fmt(f),
            BowHolder::Borrowed(t) => t.fmt(f),
            BowHolder::Static(s) => s.fmt(f),
        }
    }
}