//! [`Bow`] — a "borrowed-or-boxed" pointer: the `Box<T>` counterpart of
//! [`std::borrow::Cow`].
//!
//! `Cow<'a, T>` stores its owned side inline as `T`, so a recursive type
//! like `PyroType` can't hold `Cow<'static, PyroType>` (infinite size).
//! `Bow` stores the owned side boxed instead, and — like `Cow` — its
//! `Borrowed` variant can be built at const-eval time, which is what lets a
//! `PyroType` be a `const`.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

pub enum Bow<'a, T: 'a> {
    Borrowed(&'a T),
    Owned(Box<T>),
}

impl<'a, T> Bow<'a, T> {
    /// Box `value` into an owned `Bow`.
    pub fn new(value: T) -> Self {
        Bow::Owned(Box::new(value))
    }

    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Bow::Borrowed(_))
    }

    pub const fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }
}

impl<'a, T: Clone> Bow<'a, T> {
    /// Mutable access, cloning a borrowed value into a box first.
    pub fn to_mut(&mut self) -> &mut T {
        if let Bow::Borrowed(b) = *self {
            *self = Bow::new(b.clone());
        }
        match self {
            Bow::Owned(b) => b,
            Bow::Borrowed(_) => unreachable!(),
        }
    }

    /// The owned value, cloning if borrowed.
    pub fn into_owned(self) -> T {
        match self {
            Bow::Borrowed(b) => b.clone(),
            Bow::Owned(b) => *b,
        }
    }

    /// The owned box, cloning if borrowed.
    pub fn into_box(self) -> Box<T> {
        match self {
            Bow::Borrowed(b) => Box::new(b.clone()),
            Bow::Owned(b) => b,
        }
    }
}

impl<T> Deref for Bow<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Bow::Borrowed(b) => b,
            Bow::Owned(b) => b,
        }
    }
}

impl<T> AsRef<T> for Bow<'_, T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T> Borrow<T> for Bow<'_, T> {
    fn borrow(&self) -> &T {
        self
    }
}

impl<'a, T: Clone> Clone for Bow<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Bow::Borrowed(b) => Bow::Borrowed(b),
            Bow::Owned(b) => Bow::Owned(b.clone()),
        }
    }
}

impl<T> From<T> for Bow<'_, T> {
    fn from(value: T) -> Self {
        Bow::new(value)
    }
}

impl<T> From<Box<T>> for Bow<'_, T> {
    fn from(value: Box<T>) -> Self {
        Bow::Owned(value)
    }
}

impl<'a, T> From<&'a T> for Bow<'a, T> {
    fn from(value: &'a T) -> Self {
        Bow::Borrowed(value)
    }
}

impl<T: fmt::Debug> fmt::Debug for Bow<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for Bow<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: PartialEq> PartialEq for Bow<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: Eq> Eq for Bow<'_, T> {}

impl<T: PartialOrd> PartialOrd for Bow<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: Ord> Ord for Bow<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: Hash> Hash for Bow<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for Bow<'_, T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Bow<'_, T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Box::<T>::deserialize(deserializer).map(Bow::Owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BORROWED: Bow<'static, u32> = Bow::Borrowed(&7);

    #[test]
    fn borrowed_and_owned_compare_equal() {
        assert_eq!(BORROWED, Bow::new(7));
        assert!(BORROWED.is_borrowed());
    }

    #[test]
    fn to_mut_promotes_borrowed() {
        let mut b = BORROWED;
        *b.to_mut() += 1;
        assert!(b.is_owned());
        assert_eq!(*b, 8);
    }
}
