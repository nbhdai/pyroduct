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
use std::panic::Location;

use crate::captured::{ErrorKind, ErrorPayload};
use crate::header::{BridgeHeader, BridgeHeaderMut};
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
        if self.status() == Ok(DataStatus::ValidData) {
            let buf = self.unchecked_parse::<T>()?;
            Ok(buf)
        } else {
            Err(self.parse_as_error())
        }
    }

        pub fn parse_result<T, E>(self) -> Result<Result<TypedBuf<T>, TypedBuf<E>>, BridgeError>
    where
        // T: Success Type Constraints
        T: Archive + Bridgeable,
        T::Archived: for<'a> CheckBytes<
            Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>,
        >,
        E: Archive + Bridgeable,
        E::Archived: for<'a> CheckBytes<
            Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>,
        >,
    {
        match self.status() {
            Ok(DataStatus::ValidData) => {
                let buf = self.unchecked_parse::<T>()?;
                Ok(Ok(buf))
            }
            Ok(DataStatus::UserError) => {
                let buf = self.unchecked_parse::<E>()?;
                Ok(Err(buf))
            }
            _ => Err(self.parse_as_error())
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
        let archived_ref = rkyv::access::<T::Archived, RancorError>(slice)
            .map_err(|e| BridgeError::validation(e))?;

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

            rkyv::api::serialize_using::<_, RancorError>(value, &mut inner)
                .map_err(|e| BridgeError::serialization(e))?;

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
                e.set_status(DataStatus::UserError);
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
    /// Parse this BridgeVec as an error based on its status code.
    pub fn parse_as_error(self) -> BridgeError {
        match self.status() {
            Ok(DataStatus::ValidData) => BridgeError::UserSuccess(self),
            Ok(DataStatus::UserError) => BridgeError::UserError(ErrorVec(self)),

            // Remote errors (150-156, 3)
            Ok(DataStatus::CodeError) => BridgeError::CodePanic(self.extract_captured_error()),
            Ok(DataStatus::RemoteSerialization) => BridgeError::remote(ErrorKind::Serialization(
                ErrorPayload::Captured(self.extract_captured_error()),
            )),
            Ok(DataStatus::RemoteValidation) => BridgeError::remote(ErrorKind::Serialization(
                ErrorPayload::Captured(self.extract_captured_error()),
            )),
            Ok(DataStatus::RemoteDeserialization) => BridgeError::remote(
                ErrorKind::Deserialization(ErrorPayload::Captured(self.extract_captured_error())),
            ),
            Ok(DataStatus::RemoteTransport) => BridgeError::remote(ErrorKind::Transport(
                ErrorPayload::Captured(self.extract_captured_error()),
            )),
            Ok(DataStatus::RemoteIo) => BridgeError::remote_io(self.extract_captured_error()),
            Ok(DataStatus::RemoteUtf8) => BridgeError::remote_utf8(self.extract_captured_error()),
            Ok(DataStatus::RemoteUnexpectedEof) => BridgeError::remote(ErrorKind::InvalidHeader),
            Ok(DataStatus::RemoteInvalidHeader) => BridgeError::remote(ErrorKind::InvalidHeader),
            Ok(DataStatus::RemoteLayoutError) => BridgeError::remote(ErrorKind::LayoutError),

            // Local errors (100-109) - shouldn't normally appear in received data
            // but handle them for completeness
            Ok(DataStatus::LocalSerialization) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::local(ErrorKind::Serialization(ErrorPayload::Message(msg)))
            }
            Ok(DataStatus::LocalValidation) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::local(ErrorKind::Serialization(ErrorPayload::Message(msg)))
            }
            Ok(DataStatus::LocalDeserialization) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::local(ErrorKind::Deserialization(ErrorPayload::Message(msg)))
            }
            Ok(DataStatus::LocalTransport) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::local(ErrorKind::Transport(ErrorPayload::Message(msg)))
            }
            Ok(DataStatus::LocalIo) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::local(ErrorKind::Transport(ErrorPayload::Message(format!(
                    "I/O: {}",
                    msg
                ))))
            }
            Ok(DataStatus::LocalUtf8) => {
                let msg = String::from_utf8_lossy(self.as_slice()).to_string();
                BridgeError::local(ErrorKind::Transport(ErrorPayload::Message(format!(
                    "UTF-8: {}",
                    msg
                ))))
            }
            Ok(DataStatus::LocalInvalidHeader) => BridgeError::local(ErrorKind::InvalidHeader),
            Ok(DataStatus::LocalLayoutError) => BridgeError::local(ErrorKind::LayoutError),
            Ok(DataStatus::LocalUnexpectedEof) => BridgeError::local(ErrorKind::UnexpectedEof),

            Err(code) => BridgeError::UnknownStatus(code, self),
        }
    }

    /// Helper to deserialize a CapturedError from the payload (JSON).
    /// Falls back to a generic error if JSON deserialization fails.
    fn extract_captured_error(&self) -> Box<CapturedError> {
        if let Ok(captured) = serde_json::from_slice::<CapturedError>(self.as_slice()) {
            Box::new(captured)
        } else {
            Box::new(CapturedError {
                message: String::from_utf8_lossy(self.as_slice()).to_string(),
                file: "unknown".to_string(),
                line: 0,
                column: 0,
                error: Some("Failed to deserialize error details".into()),
                cause: None,
                stack_trace: None,
                library: None,
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
        if self.0.status() == Ok(DataStatus::ValidData) {
            let buf = self.0.unchecked_parse::<T>()?;
            Ok(buf)
        } else {
            Err(self.0.parse_as_error())
        }
    }
}

impl BridgeError {
    // Users should use the ffi code that does this for them, or should read the codebase to understand.
    #[doc(hidden)]
    #[track_caller]
    pub fn encode(&self) -> BridgeVec {
        match self {
            BridgeError::UserError(_) => {
                let mut err_vec=  CapturedError::new("Encoding a BridgeError that is an unhandled user deserialization error, not a bridge error").with_location(Location::caller()).encode();
                err_vec.set_status(DataStatus::CodeError);
                err_vec
            }
            BridgeError::UserSuccess(_) => {
                let mut err_vec=  CapturedError::new("Encoding a BridgeError that is an unhandled user deserialization error, not a bridge error").with_location(Location::caller()).encode();
                err_vec.set_status(DataStatus::CodeError);
                err_vec
            }
            BridgeError::CodePanic(err) => {
                let mut vec = err.encode();
                vec.set_status(DataStatus::CodeError);
                vec
            }
            BridgeError::UnknownStatus(_, _) => {
                let mut err_vec=  CapturedError::new("Encoding a BridgeError that is an unhandled user deserialization error, not a bridge error").with_location(Location::caller()).encode();
                err_vec.set_status(DataStatus::CodeError);
                err_vec
            }
            BridgeError::Header(error) => {
                let mut err_vec=  CapturedError::new(error.to_string()).with_location(Location::caller()).encode();
                err_vec.set_status(DataStatus::RemoteInvalidHeader);
                err_vec
            }

            BridgeError::Bridge { kind, .. } => {
                let error: Option<Box<CapturedError>> = match kind {
                    ErrorKind::Serialization(error_payload) => Some(error_payload.into()),
                    ErrorKind::Deserialization(error_payload) => Some(error_payload.into()),
                    ErrorKind::Validation(error_payload) => Some(error_payload.into()),
                    ErrorKind::Transport(error_payload) => Some(error_payload.into()),
                    ErrorKind::Io(io_payload) => Some(io_payload.into()),
                    ErrorKind::Utf8(utf8_payload) => Some(utf8_payload.into()),
                    ErrorKind::InvalidHeader => None,
                    ErrorKind::LayoutError => None,
                    ErrorKind::UnexpectedEof => None,
                };
                let status_code = kind.to_status().to_remote();

                let mut vec = match error {
                    Some(error) => error.encode(),
                    None => BridgeVec::ok(),
                };

                vec.set_status(status_code);
                vec
            }
        }
    }
}

impl<T: Archive> TypedBuf<T> {
    pub unsafe fn from_raw(pointer: *const u8) -> Result<TypedBuf<T>, BridgeError>
    where
        T: Bridgeable,
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
        rkyv::deserialize::<T, RancorError>(self.archived)
            .map_err(|e| BridgeError::deserialization(e))
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
