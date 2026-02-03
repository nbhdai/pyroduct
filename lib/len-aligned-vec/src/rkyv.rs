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

impl<E> Writer<E> for LenAlignedVec {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
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
            for<'a, 'b> T: rkyv::Serialize<
                Strategy<
                    Serializer<&'a mut LenAlignedVec, &'b mut Arena, &'b mut Share>,
                    Error
                >
            >,
        {
            let mut vec = Self::with_capacity(256);

            SCRATCH.with(|scratch| {
                let mut borrow = scratch.borrow_mut();
                let (arena, share) = &mut *borrow;

                arena.acquire();
                share.clear();

                let mut inner = Serializer::new(&mut vec, arena, share);
                
                rkyv::api::serialize_using::<_, Error>(
                    value, 
                    &mut inner
                )?;

                Ok(())
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

#[cfg(test)]
mod rkyv_tests {
    use crate::LenAlignedVec;
    use rkyv::{Archive, Serialize, Deserialize};

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        a: u32,
        b: u64,
        c: String,
    }

    #[test]
    fn test_serialization_alignment_integrity() {
        let val = TestStruct {
            a: 42,
            b: 1337,
            c: "Hello Rkyv".to_string(),
        };

        // 1. Serialize
        let vec = LenAlignedVec::serialize_from(&val).expect("Serialization failed");
        let correct_vec = rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("Serialization failed");
        
        // 2. Check alignment of the data pointer
        let data_addr = vec.data_ptr() as usize;
        assert_eq!(data_addr % 16, 0, "Data payload must start on 16-byte boundary");

        // 3. Debug: Print bytes to see where the root is
        println!("Vec: {:?}", vec);
        println!("Bytes: {:x?}", vec.as_slice());

        println!("Correct Buffer len: {}", correct_vec.len());
        println!("Correct Bytes: {:x?}", correct_vec.as_slice());

        // 4. Attempt access (This mimics what ffi.rs does)
        let slice = vec.as_slice();
        let access_result = rkyv::access::<ArchivedTestStruct, rkyv::rancor::Error>(slice);
        
        if let Err(e) = &access_result {
            panic!("Access failed! This confirms rkyv.rs logic is broken. Error: {:?}", e);
        }
    }

    #[test]
    fn test_minimal_alignment_failure() {
        // This tests if small types (like u16) cause alignment issues when serialized
        // into our 16-byte aligned buffer at offset 16.
        let val: u16 = 0xABCD;
        let vec = LenAlignedVec::serialize_from(&val).expect("Failed to serialize u16");
        
        let slice = vec.as_slice();
        let res = rkyv::access::<rkyv::Archived<u16>, rkyv::rancor::Error>(slice);
        assert!(res.is_ok(), "Small type access failed: {:?}", res.err());
    }

    #[test]
    fn test_typed_buf_parse_logic() {
        let val = TestStruct {
            a: 1,
            b: 2,
            c: "typed".to_string(),
        };

        let vec = LenAlignedVec::serialize_from(&val).unwrap();
        
        // This calls the logic inside parse()
        let typed_res = vec.parse::<TestStruct>();
        assert!(typed_res.is_ok(), "parse<T> failed: {:?}", typed_res.err());
    }
}