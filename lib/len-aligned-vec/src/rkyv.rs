use rkyv::rancor::{Error, Fallible};
use rkyv::ser::allocator::Arena;
use rkyv::ser::{Positional, Writer};


use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use rkyv::{
    Archive, Deserialize,
    bytecheck::CheckBytes,
    de::Pool,
    rancor::Strategy,
    ser::{Serializer, sharing::Share},
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};

// Define thread-local scratch space to reuse allocations.
thread_local! {
    static SCRATCH: RefCell<(Arena, Share)> = RefCell::new((Arena::new(), Share::new()));
}

use crate::LenAlignedVec;

impl Fallible for LenAlignedVec {
    type Error = Error;
}

impl Positional for LenAlignedVec {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl Writer<Error> for LenAlignedVec {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

/// A type-safe wrapper around a LenAlignedVec containing an archived rkyv type.
pub struct TypedBuf<T>
    where T: Archive,
    <T as Archive>::Archived: 'static
{
    vec: LenAlignedVec,
    archived: &'static T::Archived,
}

impl LenAlignedVec {
    /// Validates the buffer as containing a rooted `T` and returns a wrapper
    /// holding both the buffer and the typed reference.
    ///
    /// # Implementation Note
    /// This consumes the `LenAlignedVec`. The internal `archived` reference is
    /// safely tied to the stable heap allocation of the `LenAlignedVec`.
    pub fn parse<T>(self) -> Result<TypedBuf<T>, Error>
    where
        T: Archive,
        // Constraint: Validation logic
        T::Archived: for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, Error>>,
    {
        // 1. Get the slice of the payload
        let slice = self.as_slice();
        let archived_ref = rkyv::access::<T::Archived, Error>(slice)?;

        // 3. Extend lifetime to 'static.
        //    SAFETY: 
        //    - `LenAlignedVec` data is allocated on the heap via `alloc`.
        //    - Moving `self` into `TypedBuf` only moves the pointer (struct), not the heap data.
        //    - The heap address remains stable.
        //    - `TypedBuf` owns `vec` and does not expose mutable access to it, preventing reallocation.
        //    - Therefore, the reference into `vec` is valid as long as `TypedBuf` exists.
        let archived_static = unsafe { mem::transmute::<&T::Archived, &'static T::Archived>(archived_ref) };

        Ok(TypedBuf {
            vec: self,
            archived: archived_static,
        })
    }

    /// Serializes a value into a new LenAlignedVec.
    ///
    /// This uses a default `Arena` allocator and `Share` strategy (for handling 
    /// shared pointers/cycles), similar to `rkyv::to_bytes`.
    pub fn serialize_from<T>(value: &T) -> Result<Self, Error>
    where
        T: rkyv::Archive,
        // The constraints must now accept mutable references to the Arena and Share
        // because we are borrowing them from the TLS rather than passing by value.
        for<'a, 'b> T: rkyv::Serialize<
            Strategy<
                Serializer<&'a mut LenAlignedVec, &'b mut Arena, &'b mut Share>,
                Error
            >
        >,
    {
        // 1. Create the destination buffer
        let mut vec = Self::with_capacity(256);

        // 2. Access the thread-local scratch space
        SCRATCH.with(|scratch| {
            let mut borrow = scratch.borrow_mut();
            let (arena, share) = &mut *borrow;

            // IMPORTANT: Reset state to reuse capacity but clear logic from previous runs.
            // If we don't clear `share`, it might incorrectly detect cycles from previous objects.
            arena.acquire();
            share.clear();

            // 3. Construct Serializer using *mutable references* to the scratch space
            let mut inner = Serializer::new(&mut vec, arena, share);
            let strategy = Strategy::<_, Error>::wrap(&mut inner);

            value.serialize(strategy)
        })?;

        Ok(vec)
    }
}

impl<T: Archive> TypedBuf<T> {
    /// Returns a reference to the archived type.
    pub fn get(&self) -> &T::Archived {
        self.archived
    }

    /// Deserializes the archived data back into the native type T.
    pub fn deserialize(&self) -> Result<T, Error>
    where
        // Constraint: Deserialization logic
        T::Archived: Deserialize<T, Strategy<Pool, Error>>,
    {
        // rkyv::deserialize takes the archived reference and a deserializer (strategy).
        // We use the default generic deserializer strategy (Pool + Error).
        rkyv::deserialize::<T, Error>(self.archived)
    }

    /// Extract the underlying LenAlignedVec, discarding the type information.
    pub fn into_inner(self) -> LenAlignedVec {
        self.vec
    }
}

impl<T> Deref for TypedBuf<T>
    where T: Archive,
    <T as Archive>::Archived: 'static
{
    type Target = T::Archived;

    fn deref(&self) -> &Self::Target {
        &self.archived
    }
}