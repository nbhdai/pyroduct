use rkyv::rancor::{Error as RancorError, Fallible};
use rkyv::ser::allocator::{Arena, ArenaHandle};
use rkyv::ser::{Positional, Writer};

use rkyv::{
    Archive, Deserialize,
    bytecheck::CheckBytes,
    de::Pool,
    rancor::Strategy,
    ser::{Serializer, sharing::Share},
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};
use std::cell::RefCell;
use std::mem;
use std::ops::Deref;

use crate::{BridgeError, Bridgeable, CapturedError, DataStatus, ErrorVec};

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
        T::Archived: for<'a> CheckBytes<
            Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>,
        >,
    {
        if self.parsed_status() == Ok(DataStatus::ValidData) {
            let buf = self.unchecked_parse::<T>()?;
            Ok(buf)
        } else {
            Err(self.parse_as_error())
        }
    }

    /// Validates the buffer as containing a rooted `T` and returns a wrapper
    /// holding both the buffer and the typed reference.
    ///
    /// # Implementation Note
    /// This consumes the `BridgeVec`. The internal `archived` reference is
    /// safely tied to the stable heap allocation of the `BridgeVec`.
    pub fn unchecked_parse<T>(self) -> Result<TypedBuf<T>, BridgeError>
    where
        T: Archive,
        // Constraint: Validation logic
        T::Archived: for<'a> CheckBytes<
            Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>,
        >,
    {
        // 1. Get the slice of the payload
        let slice = self.as_slice();
        let archived_ref = rkyv::access::<T::Archived, RancorError>(slice).map_err(|e| BridgeError::Validation(e))?;

        // 3. Extend lifetime to 'static.
        //    SAFETY:
        //    - `BridgeVec` data is allocated on the heap via `alloc`.
        //    - Moving `self` into `TypedBuf` only moves the pointer (struct), not the heap data.
        //    - The heap address remains stable.
        //    - `TypedBuf` owns `vec` and does not expose mutable access to it, preventing reallocation.
        //    - Therefore, the reference into `vec` is valid as long as `TypedBuf` exists.
        let archived_static =
            unsafe { mem::transmute::<&T::Archived, &'static T::Archived>(archived_ref) };

        Ok(TypedBuf {
            vec: self,
            archived: archived_static,
        })
    }

    /// Serializes a value into a new BridgeVec.
    ///
    /// This uses a default `Arena` allocator and `Share` strategy (for handling
    /// shared pointers/cycles), similar to `rkyv::to_bytes`.
    pub fn serialize_from<T>(value: &T) -> Result<Self, BridgeError>
    where
        T: rkyv::Archive,
        for<'a> T: rkyv::Serialize<
                Strategy<Serializer<&'a mut BridgeVec, ArenaHandle<'a>, Share>, RancorError>,
            >,
    {
        let mut vec = Self::with_capacity(256);

        SCRATCH.with(|scratch| {
            let mut borrow = scratch.borrow_mut();
            let arena = &mut *borrow;

            let handle = arena.acquire();
            let share = Share::new();

            let mut inner = Serializer::new(&mut vec, handle, share);

            rkyv::api::serialize_using::<_, RancorError>(value, &mut inner).map_err(|e| BridgeError::Serialization(e))?;

            Ok::<(), BridgeError>(())
        })?;

        Ok(vec)
    }

    pub fn serialize_result<T, E>(result: &Result<T, E>) -> Result<BridgeVec, BridgeError>
    where
        T: Bridgeable,
        E: Bridgeable,
    {
        match &result {
            Ok(ok_value) => ok_value.serialize(),
            Err(err_value) => err_value.serialize().map(|mut e| {
                e.set_status(DataStatus::UserError as u8);
                e.set_error_version(e.version());
                e.set_version(0);
                e
            }),
        }
    }

        /// Consumes the BridgeVec and converts it into the appropriate BridgeError
    /// based on the Status header.
    /// 
    /// If the status implies a payload (e.g., CodeError/Panic), this attempts
    /// to deserialize the payload as a JSON `CapturedError`.
    pub fn parse_as_error(self) -> BridgeError {
        match self.parsed_status() {
            // Status 0: Valid Data (But we are parsing as error, so this is UserSuccess)
            Ok(DataStatus::ValidData) => BridgeError::UserSuccess(self),
            
            // Status 1: User Error
            Ok(DataStatus::UserError) => BridgeError::UserError(ErrorVec(self)),

            // --- Remote Execution Errors (JSON Payload) ---
            Ok(DataStatus::CodeError) => BridgeError::RemotePanic(self.extract_captured_error()),
            Ok(DataStatus::RemoteSerialization) => BridgeError::RemoteSerialization(self.extract_captured_error()),
            Ok(DataStatus::RemoteDeserialization) => BridgeError::RemoteDeserialization(self.extract_captured_error()),
            Ok(DataStatus::RemoteTransport) => BridgeError::RemoteTransport(self.extract_captured_error()),

            // --- Remote Protocol Errors (Empty Payload) ---
            Ok(DataStatus::RemoteNullPointer) => BridgeError::RemoteNullPointer,
            Ok(DataStatus::RemoteMisalignedPointer) => BridgeError::RemoteMisalignedPointer,
            Ok(DataStatus::RemoteInvalidHeader) => BridgeError::RemoteInvalidHeader,
            Ok(DataStatus::RemoteLayoutError) => BridgeError::RemoteLayoutError,

            // --- Local Errors (Self-reported) ---
            Ok(DataStatus::LocalSerialization) | Ok(DataStatus::LocalDeserialization) => {
                // If we have a local rancor error stored in the vec, we might need a way to extract it,
                // otherwise we return a generic Transport error with the string payload.
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::Transport(msg)
            },
            Ok(DataStatus::LocalTransport) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::Transport(msg)
            },
            
            // These shouldn't logically exist *inside* a valid BridgeVec, 
            // but if we serialized them for transport:
            Ok(DataStatus::LocalNullPointer) => BridgeError::NullPointer,
            Ok(DataStatus::LocalMisalignedPointer) => BridgeError::MisalignedPointer,
            Ok(DataStatus::LocalInvalidHeader) => BridgeError::InvalidHeader,
            Ok(DataStatus::LocalLayoutError) => BridgeError::LayoutError,
            Ok(DataStatus::LocalUnexpectedEof) => BridgeError::UnexpectedEof,
            
            // Local Io/Utf8/etc
            Ok(_) => {
                 let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                 BridgeError::Transport(format!("Unhandled local error status: {}", msg))
            }

            // Unknown
            Err(code) => BridgeError::UnknownStatus(code, self),
        }
    }

    /// Helper to deserialize a CapturedError from the payload (JSON).
    /// Falls back to a generic error if JSON deserialization fails.
    fn extract_captured_error(&self) -> Box<CapturedError> {
        // Try to deserialize JSON
        if let Ok(captured) = serde_json::from_slice::<CapturedError>(self.as_slice()) {
            Box::new(captured)
        } else {
            // Fallback if the payload isn't valid JSON (e.g., raw string panic)
            Box::new(CapturedError {
                message: String::from_utf8_lossy(self.as_slice()).to_string(),
                file: "unknown".to_string(),
                line: 0,
                column: 0,
                error: Some("Failed to deserialize error details".into()),
                cause: None,
            })
        }
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
        T::Archived: for<'a> CheckBytes<
            Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>,
        >,
    {
        if self.0.parsed_status() == Ok(DataStatus::ValidData) {
            let buf = self.0.unchecked_parse::<T>()?;
            Ok(buf)
        } else {
            Err(self.0.parse_as_error())
        }
    }
}


impl BridgeError {
    pub fn encode(&self) -> BridgeVec {
        match self {
            BridgeError::UserError(err_vec) => err_vec.0.clone(),
            BridgeError::UserSuccess(vec) => vec.clone(),
            
            err => {
                // 1. Assign Status Code
                // If we are serializing "NullPointer", it means WE (the remote) found it,
                // so we send it back as "RemoteNullPointer" (153).
                let (status_code, is_panic) = match err {
                    // Specific Remote Infrastructure Errors
                    BridgeError::NullPointer => (153, false),
                    BridgeError::MisalignedPointer => (154, false),
                    BridgeError::InvalidHeader => (155, false),
                    BridgeError::LayoutError => (156, false),
                    
                    // Remote Execution Errors
                    BridgeError::RemoteError(_) => (150, true), // Explicit Panic
                    BridgeError::Serialization(_) => (151, false),
                    BridgeError::Transport(_) => (152, false),

                    // Generic Fallback
                    _ => (152, false), 
                };

                // 2. Capture Stack Trace 
                let detailed = if let BridgeError::RemoteError(boxed) = err {
                    *boxed.clone()
                } else {
                    let location = std::panic::Location::caller();
                    let backtrace = std::backtrace::Backtrace::capture();
                    CapturedError {
                        message: err.to_string(),
                        file: location.file().to_string(),
                        line: location.line(),
                        column: location.column(),
                        error: Some(format!("{:?}", err)),
                        cause: if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                            Some(backtrace.to_string())
                        } else { None },
                    }
                };

                // 3. Wrap in FfiError
                // We use Generic for infrastructure errors because the Status Code 
                // carries the specific type information (Null/Misaligned).
                let transport_wrapper = if is_panic {
                    FfiError::Panic(Box::new(detailed))
                } else {
                    // For NullPointer (153), etc., we send the detailed trace inside a Generic wrapper.
                    FfiError::Generic(serde_json::to_string(&detailed).unwrap_or_default())
                };

                // 4. Create BridgeVec
                let mut vec = BridgeVec::from_transport_error(&transport_wrapper);
                vec.set_status(status_code);
                vec
            }
        }
    }
}

impl<T: Archive> TypedBuf<T> {
    pub unsafe fn from_raw(
        pointer: *const u8,
    ) -> Result<TypedBuf<T>, BridgeError> 
    where T: Bridgeable
    {
        let vec = unsafe { BridgeVec::from_raw(pointer) }?;
        T::parse(vec)
    }

    /// Returns a reference to the archived type.
    pub fn get(&self) -> &T::Archived {
        self.archived
    }

    /// Deserializes the archived data back into the native type T.
    pub fn deserialize(&self) -> Result<T, BridgeError>
    where
        // Constraint: Deserialization logic
        T::Archived: Deserialize<T, Strategy<Pool, RancorError>>,
    {
        // rkyv::deserialize takes the archived reference and a deserializer (strategy).
        // We use the default generic deserializer strategy (Pool + Error).
        rkyv::deserialize::<T, RancorError>(self.archived).map_err(|e| BridgeError::Deserialization(e))
    }

    /// Extract the underlying BridgeVec, discarding the type information.
    pub fn into_inner(self) -> BridgeVec {
        self.vec
    }
}

impl<T> Deref for TypedBuf<T>
where
    T: Archive,
    <T as Archive>::Archived: 'static,
{
    type Target = T::Archived;

    fn deref(&self) -> &Self::Target {
        &self.archived
    }
}

#[cfg(test)]
mod rkyv_tests {
    use crate::BridgeVec;
    use rkyv::{Archive, Deserialize, Serialize};

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
        let correct_vec =
            rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("Serialization failed");

        // 2. Check alignment of the data pointer
        let data_addr = vec.data_ptr() as usize;
        assert_eq!(
            data_addr % 16,
            0,
            "Data payload must start on 16-byte boundary"
        );

        // 3. Debug: Print bytes to see where the root is
        println!("Vec: {:?}", vec);
        println!("Bytes: {:x?}", vec.as_slice());

        println!("Correct Buffer len: {}", correct_vec.len());
        println!("Correct Bytes: {:x?}", correct_vec.as_slice());

        // 4. Attempt access (This mimics what ffi.rs does)
        let slice = vec.as_slice();
        let access_result = rkyv::access::<ArchivedTestStruct, rkyv::rancor::Error>(slice);

        if let Err(e) = &access_result {
            panic!(
                "Access failed! This confirms rkyv.rs logic is broken. Error: {:?}",
                e
            );
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
