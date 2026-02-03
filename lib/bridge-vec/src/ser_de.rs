use rkyv::rancor::{Error as RancorError, Fallible};
use rkyv::ser::allocator::{Arena, ArenaHandle};
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

use crate::{BridgeError, DataStatus, ErrorVec};

// Define thread-local scratch space to reuse allocations.
thread_local! {
    static SCRATCH: RefCell<Arena> = RefCell::new(Arena::new());
}

use crate::{BridgeVec, TypedBuf};

impl Fallible for BridgeVec {
    type Error = RancorError;
}

impl Positional for BridgeVec {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl<E> Writer<E> for BridgeVec {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

impl BridgeVec {
    /// Verifies the bridge status header and reconstructs the appropriate result.
    ///
    /// - **Status 0 (ValidData)**: Returns `Ok(Ok(TypedBuf<T>))` (Zero-copy).
    /// - **Status 1 (UserError)**: Returns `Ok(Err(TypedBuf<E>))` (Zero-copy).
    /// - **Status 2 (TransportError)**: Deserializes JSON error and returns `Err(BridgeError::Ffi(...))`.
    /// - **Status 3 (Utf8Error)**: Returns `Err(BridgeError::RemoteError(...))`.
    pub fn parse<T>(self) -> Result<TypedBuf<T>, BridgeError>
    where
        // T: Success Type Constraints
        T: Archive,
        T::Archived: for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>>,
    {
        if self.parsed_status() == Ok(DataStatus::ValidData) {
                let buf = self.unchecked_parse::<T>()?;
                Ok(buf)
        } else {
            Err(self.parse_as_error())
        }
    }

    pub fn parse_as_error(self) -> BridgeError {
        match self.parsed_status() {
            Ok(DataStatus::ValidData) => BridgeError::UserSuccess(self),
            Ok(DataStatus::UserError) => BridgeError::UserError(ErrorVec(self)),
            Ok(DataStatus::TransportError) => {
                let slice = self.as_slice();
                match serde_json::from_slice::<serde_json::Value>(slice) {
                    Ok(transport) => BridgeError::Transport(transport),
                    Err(e) => BridgeError::RemoteError(format!("Failed to parse error JSON: {}", e)),
                }
            }
            Ok(DataStatus::Utf8Error) => {
                match std::str::from_utf8(self.as_slice()) {
                    Ok(s) => BridgeError::RemoteError(s.to_string()),
                    Err(s) => BridgeError::Utf8(s),
                }
                
            }
            Err(unknown) => BridgeError::UnknownStatus(unknown, self),
        }
    }

    /// Validates the buffer as containing a rooted `T` and returns a wrapper
    /// holding both the buffer and the typed reference.
    ///
    /// # Implementation Note
    /// This consumes the `BridgeVec`. The internal `archived` reference is
    /// safely tied to the stable heap allocation of the `BridgeVec`.
    pub fn unchecked_parse<T>(self) -> Result<TypedBuf<T>, RancorError>
    where
        T: Archive,
        // Constraint: Validation logic
        T::Archived: for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>>,
    {
        // 1. Get the slice of the payload
        let slice = self.as_slice();
        let archived_ref = rkyv::access::<T::Archived, RancorError>(slice)?;

        // 3. Extend lifetime to 'static.
        //    SAFETY: 
        //    - `BridgeVec` data is allocated on the heap via `alloc`.
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

    /// Serializes a value into a new BridgeVec.
    ///
    /// This uses a default `Arena` allocator and `Share` strategy (for handling 
    /// shared pointers/cycles), similar to `rkyv::to_bytes`.
    pub fn serialize_from<T>(value: &T) -> Result<Self, RancorError>
        where
            T: rkyv::Archive,
            for<'a> T: rkyv::Serialize<
                Strategy<
                    Serializer<&'a mut BridgeVec, ArenaHandle<'a>, Share>,
                    RancorError
                >
            >,
        {
            let mut vec = Self::with_capacity(256);

            SCRATCH.with(|scratch| {
                let mut borrow = scratch.borrow_mut();
                let arena = &mut *borrow;

                let handle = arena.acquire();
                let share = Share::new();

                let mut inner = Serializer::new(&mut vec, handle, share);
                
                rkyv::api::serialize_using::<_, RancorError>(
                    value, 
                    &mut inner
                )?;

                Ok(())
            })?;

            Ok(vec)
        }
}


impl ErrorVec {
    /// Verifies the bridge status header and reconstructs the appropriate result.
    ///
    /// - **Status 0 (ValidData)**: Returns `Ok(Ok(TypedBuf<T>))` (Zero-copy).
    /// - **Status 1 (UserError)**: Returns `Ok(Err(TypedBuf<E>))` (Zero-copy).
    /// - **Status 2 (TransportError)**: Deserializes JSON error and returns `Err(BridgeError::Ffi(...))`.
    /// - **Status 3 (Utf8Error)**: Returns `Err(BridgeError::RemoteError(...))`.
    pub fn parse<T>(self) -> Result<TypedBuf<T>, BridgeError>
    where
        // T: Success Type Constraints
        T: Archive,
        T::Archived: for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>>,
    {
        if self.0.parsed_status() == Ok(DataStatus::ValidData) {
                let buf = self.0.unchecked_parse::<T>()?;
                Ok(buf)
        } else {
            Err(self.0.parse_as_error())
        }
    }
}


impl<T: Archive> TypedBuf<T> {
    /// Returns a reference to the archived type.
    pub fn get(&self) -> &T::Archived {
        self.archived
    }

    /// Deserializes the archived data back into the native type T.
    pub fn deserialize(&self) -> Result<T, RancorError>
    where
        // Constraint: Deserialization logic
        T::Archived: Deserialize<T, Strategy<Pool, RancorError>>,
    {
        // rkyv::deserialize takes the archived reference and a deserializer (strategy).
        // We use the default generic deserializer strategy (Pool + Error).
        rkyv::deserialize::<T, RancorError>(self.archived)
    }

    /// Extract the underlying BridgeVec, discarding the type information.
    pub fn into_inner(self) -> BridgeVec {
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
    use crate::BridgeVec;
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
        let vec = BridgeVec::serialize_from(&val).expect("Serialization failed");
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
        let vec = BridgeVec::serialize_from(&val).expect("Failed to serialize u16");
        
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

        let vec = BridgeVec::serialize_from(&val).unwrap();
        
        // This calls the logic inside parse()
        let typed_res = vec.parse::<TestStruct>();
        assert!(typed_res.is_ok(), "parse<T> failed: {:?}", typed_res.err());
    }
}