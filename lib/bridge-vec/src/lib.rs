//! # BridgeVec: Zero-Copy Transport
//!
//! `bridge_vec` provides a specialized buffer type optimized for moving complex Rust types
//! across boundaries (like FFI to dynamically loaded rust) or process boundaries with zero copy overhead.
//!
//! It is meant to provide a unified data structure to safely pass across ffi, tcp, wasm, and unix-sockets
//! to other rust libraries.
//!
//! ## How it works
//!
//! This library bridges the gap between high-level Rust types and raw memory pointers by combining
//! **`rkyv`** (for zero-copy serialization) with a **custom memory layout** that carries protocol
//! metadata (length, capacity, status codes) in a 16-byte aligned header.
//!
//! 1.  **Define**: Annotate your Rust types with `#[bridgeable]`. This derives the necessary `rkyv` traits.
//! 2.  **Serialize**: Call `.serialize()` on your type to produce a `BridgeVec`. This serializes the data
//!     directly into an FFI-safe, aligned memory buffer.
//! 3.  **Transport**: Pass the raw pointer (`vec.into_raw()`) to the foreign system.
//! 4.  **Access**: On the receiving end, the pointer is reconstructed into a `BridgeVec`. The data can
//!     then be accessed immediately (zero-copy) via `parse()`, or fully deserialized back into a Rust type.
//!
//! ### Example
//!
//! ```rust
//! use bridge_vec::{bridgeable, BridgeVec, Bridgeable};
//!
//! // 1. Define your data types
//! #[bridgeable]
//! #[derive(Debug, PartialEq)] // The macro handles rkyv implementation
//! struct UserProfile {
//!     id: u32,
//!     username: String,
//!     tags: Vec<String>,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let original = UserProfile {
//!         id: 101,
//!         username: "ferris".to_string(),
//!         tags: vec!["rust".into(), "ffi".into()],
//!     };
//!
//!     // 2. Serialize to the bridge buffer
//!     // This creates a BridgeVec with the specific header layout
//!     let bridge_vec = original.serialize()?;
//!
//!     // --- FFI BOUNDARY SIMULATION ---
//!     let pointer: *const u8 = bridge_vec.into_raw();
//!     // This validates the header
//!     let passed_vec = unsafe { BridgeVec::from_raw(pointer) }
//!         .expect("Should pass the checks as it's fresh");
//!     // -------------------------------
//!
//!     // 3. Parse back into a TypedBuf (Zero-Copy access)
//!     // This validates the buffer and gives us a view into the archived data
//!     let access = UserProfile::parse(passed_vec)?;
//!     
//!     // We can read fields without allocating new strings/vecs
//!     assert_eq!(access.id, 101);
//!     assert_eq!(access.username, "ferris");
//!
//!     // 4. (Optional) Full Deserialization
//!     // If you need the owned Rust type back:
//!     let recovered: UserProfile = access.deserialize()?;
//!     assert_eq!(original, recovered);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Results
//!
//! Results are supported with dedicated functions. `serialize_result` and `parse_result`.
//! The wire format handles this natively.
//!
//! ### Example: Result Transport
//!
//! ```rust
//! use bridge_vec::{bridgeable, BridgeVec, Bridgeable};
//!
//! // 1. Define Success Type
//! #[bridgeable]
//! #[derive(Debug, PartialEq)]
//! struct Response {
//!     id: u32,
//!     payload: String,
//! }
//!
//! // 2. Define Error Type
//! #[bridgeable]
//! #[derive(Debug, PartialEq)]
//! struct ApiError {
//!     code: u16,
//!     reason: String,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // --- Success Case ---
//!     let success: Result<Response, ApiError> = Ok(Response {
//!         id: 101,
//!         payload: "Data retrieved".to_string(),
//!     });
//!
//!     // Serialize: Sets Status = 0 (ValidData)
//!     let vec = BridgeVec::serialize_result(&success)?;
//!     assert_eq!(vec.status(), 0);
//!
//!     // Parse: Returns Result<TypedBuf<Response>, TypedBuf<ApiError>>
//!     match BridgeVec::parse_result::<Response, ApiError>(vec)? {
//!         Ok(data) => assert_eq!(data.payload, "Data retrieved"),
//!         Err(_) => panic!("Expected success"),
//!     }
//!
//!     // --- Failure Case ---
//!     let failure: Result<Response, ApiError> = Err(ApiError {
//!         code: 404,
//!         reason: "Not Found".to_string(),
//!     });
//!
//!     // Serialize: Sets Status = 1 (UserError)
//!     let vec_err = BridgeVec::serialize_result(&failure)?;
//!     assert_eq!(vec_err.status(), 1);
//!
//!     // Parse: Returns Err(TypedBuf<ApiError>)
//!     match BridgeVec::parse_result::<Response, ApiError>(vec_err)? {
//!         Ok(_) => panic!("Expected error"),
//!         Err(e) => {
//!             assert_eq!(e.code, 404);
//!             assert_eq!(e.reason, "Not Found");
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # FFI Safety & Panic Handling
//!
//! This module provides the safety layer required when crossing FFI boundaries (e.g., calling
//! into a dynamic library or from a C host). It ensures that panics in Rust code do not
//! unwind across the FFI boundary, which is undefined behavior.
//!
//! ### Features
//!
//! 1. **Panic Catching**: Wraps execution in `catch_unwind` to contain panics.
//! 2. **Rich Error Reporting**: Installs a custom panic hook to capture file, line, and message details
//!    into Thread Local Storage (TLS) before the stack unwinds.
//! 3. **Transport Error Serialization**: Converts panics or serialization failures into a
//!    `BridgeVec` with `Status::TransportError` (2), allowing the caller to receive a structured
//!    error report safely.
//!
//! ### Usage
//!
//! Use `execute_safe` to wrap any logic intended for FFI export:
//!
//! ```ignore
//! #[no_mangle]
//! pub extern "C" fn my_ffi_func() -> *mut u8 {
//!     bridge_vec::ffi::execute_safe(|| {
//!         // Your logic here
//!         process_data()
//!     }).into_raw()
//! }
//! ```
//!
//! # Memory Layout & Header Protocol
//!
//! `BridgeVec` utilizes a custom 16-byte aligned memory layout compatible with FFI
//! boundary crossing. The allocation consists of a **16-byte Header** followed immediately
//! by the **Data Payload**.
//!
//! ## Layout Diagram
//!
//! ```text
//!  Pointer (16-byte aligned)
//!  │
//!  ▼
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                             Magic (u32)                           │
//! ├───────────────────────────────────────────────────────────────────┤
//! │                              Len (u32)                            │
//! ├───────────────────────────────────────────────────────────────────┤
//! │                              Cap (u32)                            │
//! ├─────────────────┬────────────────┬──────────────────┬─────────────┤
//! │ Wire Format(u8) │ User Vers (u8) │ User Err Ver(u8) │ Status (u8) │
//! ├─────────────────┴────────────────┴──────────────────┴─────────────┤
//! │                           Data Payload ...                        │
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Header Fields
//!
//! | Offset | Type  | Field          | Description                                                        |
//! |--------|-------|----------------|--------------------------------------------------------------------|
//! | `0x00` | `u32` | Magic          | Constant `0x7079726F` (ASCII "pyro"). Verifies pointer validity.   |
//! | `0x04` | `u32` | Len            | Current length of the data payload in bytes.                       |
//! | `0x08` | `u32` | Cap            | Total allocated capacity (including header) in bytes.              |
//! | `0x0C` | `u8`  | Wire Format    | Protocol Version number                                            |
//! | `0x0C` | `u8`  | User Version   | User message Version number                                        |
//! | `0x0C` | `u8`  | User Error Ver | User Error message Version number                                  |
//! | `0x0E` | `u8`  | Status         | **Message Protocol Status**. Used to indicate the type of payload. |
//!
//! ## Status Codes (Offset 0x0E)
//!
//! When passing `Result<T, E>` across FFI or transport boundaries, the status field determines how
//! the payload should be interpreted:
//!
//! * **`0` (ValidData)**: The payload is a valid `rkyv` archived `T`. Corresponds to `Ok(T)`.
//! * **`1` (UserError)**: The payload is a valid `rkyv` archived `E`. Corresponds to `Err(E)`.
//! * **`2` (Transport Error)**: The payload is a serialized `RkyvFfiError`, or a transport error. Indicates a system failure (e.g., serialization panic, validation failure) rather than a logic error.
//!
//! //! `BridgeVec` optimizes `Result<T, E>` transport by lifting the variant discriminant into the
//! **Status** header field. This avoids the overhead of serializing the `Result` enum wrapper
//! and allows the receiving end to immediately distinguish between success and failure before
//! parsing the payload.
//!
//! - **Success (`Ok(T)`)**:
//!   - **Status**: `0` (ValidData)
//!   - **Payload**: Serialized `T`
//!   - **Version**: Uses the `User Version` field.
//!
//! - **Failure (`Err(E)`)**:
//!   - **Status**: `1` (UserError)
//!   - **Payload**: Serialized `E`
//!   - **Version**: Uses the `User Error Version` field.
//!
//! ## Do Not Use In Production (yet)
//!
//! We're going to dog food this until we get versioning correct

use std::alloc::{self, Layout};
use std::hash::Hasher;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::{fmt, io, slice};

// Re-export rkyv for users
pub use rkyv;

pub mod captured;
mod common;
pub mod ffi;
pub mod ser_de;

pub use captured::DataStatus;

pub use rkyv::rancor::Error as RancorError;

// Async is not supported for wasm
#[cfg(not(target_arch = "wasm32"))]
pub mod tokio;

// Re-export derive macro
pub use bridge_derive::bridgeable;

pub(crate) const MAGIC_VAL: u32 = 0x7079726F; // "pyro"
pub const PROTOCOL_VERSION: u8 = 1;

pub use captured::{BridgeError, CapturedError};

/// A specialized Result type for BridgeVec operations.
pub type BridgeResult<T> = Result<T, BridgeError>;

/// Trait automatically derived by `#[bridgeable]`
pub trait Bridgeable: ::rkyv::Archive + Sized {
    fn deserialize(vec: &TypedBuf<Self>) -> Result<Self, BridgeError>;
    fn serialize(&self) -> Result<BridgeVec, BridgeError>;
    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError>;

    fn parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        if vec.parsed_status() == Ok(DataStatus::ValidData) {
            let buf = Self::unchecked_parse(vec)?;
            Ok(buf)
        } else {
            Err(vec.parse_as_error())
        }
    }
    fn parse_error(vec: ErrorVec) -> Result<TypedBuf<Self>, BridgeError> {
        if vec.0.parsed_status() == Ok(DataStatus::UserError) {
            let buf = Self::unchecked_parse(vec.0)?;
            Ok(buf)
        } else {
            Err(vec.0.parse_as_error())
        }
    }
}

/// A 16-byte aligned buffer with a self-describing header.
/// Compatible with FFI passing as a raw pointer or TCP/Unix framing.
///
///
pub struct BridgeVec {
    ptr: NonNull<u8>,
}

/// A wrapper for bridge vec that means this contains a user defined error
pub struct ErrorVec(pub(crate) BridgeVec);

/// A type-safe wrapper around a BridgeVec containing an archived rkyv type.
pub struct TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    vec: BridgeVec,
    archived: &'static T::Archived,
}

// SAFETY: BridgeVec owns its allocation exclusively
// and contains no references to thread-local state
unsafe impl Send for BridgeVec {}
unsafe impl Sync for BridgeVec {}

/// A borrowed, non-owning view into a BridgeVec buffer.
/// Does not free memory on drop.
pub struct BridgeVecRef<'a> {
    ptr: *const u8,
    _marker: PhantomData<&'a [u8]>,
}

impl BridgeVec {
    pub const ALIGN: usize = 16;
    pub const HEADER_SIZE: usize = 16;

    // --- Header Layout (16 Bytes) ---
    // 0x00 - 0x03: Magic (u32)
    // 0x04 - 0x07: Len (u32)
    // 0x08 - 0x0B: Cap (u32)
    // 0x0C: Wire Format (u8)
    // 0x0D: User Version (u8)
    // 0x0E: User Error Version (u8)
    // 0x0F: Status (u8)

    const OFFSET_MAGIC: usize = 0;
    const OFFSET_LEN: usize = 4;
    const OFFSET_CAP: usize = 8;
    const OFFSET_WIRE_FORMAT: usize = 12;
    const OFFSET_USER_VERSION: usize = 13;
    const OFFSET_ERR_VERSION: usize = 14;
    const OFFSET_STATUS: usize = 15;

    /// Creates a new vector with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        // Ensure strictly aligned allocation size
        let total_cap = (capacity + Self::HEADER_SIZE).max(Self::ALIGN);

        let layout =
            Layout::from_size_align(total_cap, Self::ALIGN).expect("Invalid layout alignment");

        let ptr = unsafe {
            let raw = alloc::alloc(layout);
            if raw.is_null() {
                alloc::handle_alloc_error(layout);
            }

            // Initialize Header
            ptr::write(raw.add(Self::OFFSET_MAGIC) as *mut u32, MAGIC_VAL);
            ptr::write(raw.add(Self::OFFSET_LEN) as *mut u32, 0);
            ptr::write(raw.add(Self::OFFSET_CAP) as *mut u32, total_cap as u32);

            // Byte fields
            ptr::write(
                raw.add(Self::OFFSET_WIRE_FORMAT) as *mut u8,
                PROTOCOL_VERSION,
            );
            ptr::write(raw.add(Self::OFFSET_USER_VERSION) as *mut u8, 0);
            ptr::write(raw.add(Self::OFFSET_ERR_VERSION) as *mut u8, 0);
            ptr::write(raw.add(Self::OFFSET_STATUS) as *mut u8, 0); // Default: ValidData

            NonNull::new_unchecked(raw)
        };

        Self { ptr }
    }

    /// Reconstructs an owned Vec from a raw pointer.
    ///
    /// # Safety
    /// - `ptr` must have been created by `BridgeVec::into_raw()` or equivalent
    /// - Caller must ensure no other owner exists for this allocation
    /// - Caller transfers ownership to the returned `BridgeVec`
    pub unsafe fn from_raw(ptr: *const u8) -> Result<Self, BridgeError> {
        if ptr.is_null() {
            return Err(BridgeError::null_pointer());
        }
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err(BridgeError::misaligned_pointer());
        }

        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != MAGIC_VAL {
            return Err(BridgeError::invalid_header());
        }

        Ok(Self {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut u8) },
        })
    }

    /// Creates a non-owning borrowed view from a raw pointer.
    ///
    /// # Safety
    /// - `ptr` must point to a valid BridgeVec allocation
    /// - The allocation must remain valid for lifetime `'a`
    /// - Caller must not free or reallocate the memory during `'a`
    pub unsafe fn borrow_raw<'a>(ptr: *const u8) -> Result<BridgeVecRef<'a>, BridgeError> {
        if ptr.is_null() {
            return Err(BridgeError::null_pointer());
        }
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err(BridgeError::misaligned_pointer());
        }

        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != MAGIC_VAL {
            return Err(BridgeError::invalid_header());
        }

        Ok(BridgeVecRef {
            ptr,
            _marker: PhantomData,
        })
    }

    /// Consumes self and returns the raw pointer.
    ///
    /// Caller is responsible for eventually reconstructing via `from_raw`
    /// and dropping, or manually deallocating with the correct layout.
    pub fn into_raw(self) -> *const u8 {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    // --- Header Accessors ---

    /// Gets the status code from the header (Offset 0x0F).
    #[inline]
    pub fn status(&self) -> u8 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_STATUS) as *const u8) }
    }

    #[inline]
    pub fn parsed_status(&self) -> Result<DataStatus, u8> {
        match self.status() {
            0 => Ok(DataStatus::ValidData),
            1 => Ok(DataStatus::UserError),
            3 => Ok(DataStatus::CodeError),

            100 => Ok(DataStatus::LocalSerialization),
            101 => Ok(DataStatus::LocalDeserialization),
            102 => Ok(DataStatus::LocalTransport),
            103 => Ok(DataStatus::LocalIo),
            104 => Ok(DataStatus::LocalUtf8),
            105 => Ok(DataStatus::LocalNullPointer),
            106 => Ok(DataStatus::LocalMisalignedPointer),
            107 => Ok(DataStatus::LocalInvalidHeader),
            108 => Ok(DataStatus::LocalLayoutError),
            109 => Ok(DataStatus::LocalUnexpectedEof),

            150 => Ok(DataStatus::RemoteSerialization),
            151 => Ok(DataStatus::RemoteDeserialization),
            152 => Ok(DataStatus::RemoteTransport),

            153 => Ok(DataStatus::RemoteNullPointer),
            154 => Ok(DataStatus::RemoteMisalignedPointer),
            155 => Ok(DataStatus::RemoteInvalidHeader),
            156 => Ok(DataStatus::RemoteLayoutError),

            other => Err(other),
        }
    }

    #[doc(hidden)]
    /// Sets the status code in the header (Offset 0x0F).
    #[inline]
    pub fn set_status(&mut self, status: DataStatus) {
        self.set_status_u8(status as u8);
    }

    #[doc(hidden)]
    /// Sets the status code in the header (Offset 0x0F).
    #[inline]
    pub fn set_status_u8(&mut self, status: u8) {
        unsafe {
            ptr::write(
                self.ptr.as_ptr().add(Self::OFFSET_STATUS) as *mut u8,
                status,
            )
        }
    }

    /// Gets the User Version (Offset 0x0D).
    #[inline]
    pub fn version(&self) -> u8 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_USER_VERSION) as *const u8) }
    }

    /// Sets the User Version (Offset 0x0D).
    #[inline]
    pub fn set_version(&mut self, version: u8) {
        unsafe {
            ptr::write(
                self.ptr.as_ptr().add(Self::OFFSET_USER_VERSION) as *mut u8,
                version,
            )
        }
    }

    /// Gets the User Error Version (Offset 0x0E).
    #[inline]
    pub fn error_version(&self) -> u8 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_ERR_VERSION) as *const u8) }
    }

    /// Sets the User Error Version (Offset 0x0E).
    #[inline]
    pub fn set_error_version(&mut self, version: u8) {
        unsafe {
            ptr::write(
                self.ptr.as_ptr().add(Self::OFFSET_ERR_VERSION) as *mut u8,
                version,
            )
        }
    }

    /// Gets the Wire Format Version (Offset 0x0C).
    #[doc(hidden)]
    #[inline]
    pub fn wire_format(&self) -> u8 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_WIRE_FORMAT) as *const u8) }
    }

    #[doc(hidden)]
    #[inline]
    pub fn set_wire_format(&mut self, version: u8) {
        unsafe {
            ptr::write(
                self.ptr.as_ptr().add(Self::OFFSET_WIRE_FORMAT) as *mut u8,
                version,
            )
        }
    }

    /// Is this a Ok message?
    pub fn is_ok(&self) -> bool {
        self.status() == 0
    }

    /// Is this a error message?
    pub fn is_err(&self) -> bool {
        self.status() == 1
    }

    /// Is this a transport error?
    pub fn is_bridge_err(&self) -> bool {
        self.status() != 0 && self.status() != 1
    }

    // --- Data Accessors ---

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.as_ptr().add(Self::HEADER_SIZE) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_LEN) as *const u32) as usize }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_CAP) as *const u32) as usize }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a slice containing the Header (16 bytes) AND the Data (len bytes).
    pub fn as_packet_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), Self::HEADER_SIZE + self.len()) }
    }

    // --- Vec Operations ---

    pub fn push(&mut self, byte: u8) {
        if self.len() + Self::HEADER_SIZE == self.capacity() {
            self.grow(1);
        }

        unsafe {
            let len = self.len();
            let data_start = self.ptr.as_ptr().add(Self::HEADER_SIZE);
            ptr::write(data_start.add(len), byte);
            self.set_len(len + 1);
        }
    }

    pub fn extend_from_slice(&mut self, other: &[u8]) {
        let required = other.len();
        let current_len = self.len();
        let current_cap = self.capacity();

        if current_len + required + Self::HEADER_SIZE > current_cap {
            self.grow(required);
        }

        unsafe {
            ptr::copy_nonoverlapping(
                other.as_ptr(),
                self.ptr.as_ptr().add(Self::HEADER_SIZE + current_len),
                required,
            );
            self.set_len(current_len + required);
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr().add(Self::HEADER_SIZE), self.len()) }
    }

    pub fn clear(&mut self) {
        unsafe {
            self.set_len(0);
        }
    }

    // --- Internals ---

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            ptr::write(
                self.ptr.as_ptr().add(Self::OFFSET_LEN) as *mut u32,
                new_len as u32,
            )
        };
    }

    fn grow(&mut self, additional: usize) {
        let current_cap = self.capacity();
        let current_len = self.len();

        let required_cap = current_len
            .checked_add(Self::HEADER_SIZE)
            .and_then(|v| v.checked_add(additional))
            .expect("capacity overflow");

        let mut new_cap = current_cap.saturating_mul(2).max(required_cap);

        let remainder = new_cap % Self::ALIGN;
        if remainder != 0 {
            new_cap = new_cap
                .checked_add(Self::ALIGN - remainder)
                .expect("capacity overflow during alignment");
        }

        let old_layout = Layout::from_size_align(current_cap, Self::ALIGN).unwrap();

        unsafe {
            let new_ptr = alloc::realloc(self.ptr.as_ptr(), old_layout, new_cap);
            if new_ptr.is_null() {
                alloc::handle_alloc_error(Layout::from_size_align(new_cap, Self::ALIGN).unwrap());
            }
            ptr::write(new_ptr.add(Self::OFFSET_CAP) as *mut u32, new_cap as u32);
            self.ptr = NonNull::new_unchecked(new_ptr);
        }
    }

    /// Returns a new, owned BridgeVec representing Ok(()).
    /// This performs an allocation so it can be safely dropped later.
    pub fn ok() -> Self {
        // We cannot return a pointer to static memory because BridgeVec::drop
        // will try to deallocate it. We must allocate new memory.
        let vec = Self::with_capacity(0);

        // We explicitly overwrite the header with UNIT_BYTES to ensure
        // it matches the "official" static representation exactly.
        unsafe {
            ptr::copy_nonoverlapping(UNIT_HEADER.0.as_ptr(), vec.ptr.as_ptr(), Self::HEADER_SIZE);
        }
        vec
    }
}

// --- BridgeVecRef Implementation ---

impl<'a> BridgeVecRef<'a> {
    #[inline]
    pub fn status(&self) -> u8 {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_STATUS) as *const u8) }
    }

    #[inline]
    pub fn wire_format(&self) -> u8 {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_WIRE_FORMAT) as *const u8) }
    }

    #[inline]
    pub fn version(&self) -> u8 {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_USER_VERSION) as *const u8) }
    }

    #[inline]
    pub fn error_version(&self) -> u8 {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_ERR_VERSION) as *const u8) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_LEN) as *const u32) as usize }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_CAP) as *const u32) as usize }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.add(BridgeVec::HEADER_SIZE) }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.len()) }
    }

    pub fn as_packet_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr, BridgeVec::HEADER_SIZE + self.len()) }
    }
}

impl BridgeVecRef<'static> {
    /// Returns a static reference to the global Ok header.
    /// This is Zero-Copy and Zero-Allocation.
    pub fn ok() -> BridgeVecRef<'static> {
        // SAFETY:
        // 1. UNIT_HEADER is static, so it lives for 'static.
        // 2. UNIT_HEADER is #[repr(align(16))], satisfying alignment.
        // 3. The bytes are valid (checked by UNIT_BYTES definition).
        BridgeVecRef {
            ptr: UNIT_HEADER.0.as_ptr(),
            _marker: PhantomData,
        }
    }
}

impl<'a> fmt::Debug for BridgeVecRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeVecRef")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("usr_ver", &self.version())
            .field("err_ver", &self.error_version())
            .field("data", &self.as_slice())
            .finish()
    }
}

// --- BridgeVec Trait Implementations ---

impl Clone for BridgeVec {
    fn clone(&self) -> Self {
        let mut new_vec = Self::with_capacity(self.len());

        new_vec.extend_from_slice(self.as_slice());

        new_vec.set_status_u8(self.status());
        new_vec.set_wire_format(self.wire_format());
        new_vec.set_version(self.version());
        new_vec.set_error_version(self.error_version());

        new_vec
    }
}

impl fmt::Debug for BridgeVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeVec")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("usr_ver", &self.version())
            .field("err_ver", &self.error_version())
            .field("data", &self.as_slice())
            .finish()
    }
}

impl PartialEq for BridgeVec {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for BridgeVec {}

impl std::hash::Hash for BridgeVec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl Deref for BridgeVec {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for BridgeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for BridgeVec {
    fn drop(&mut self) {
        let cap = self.capacity();
        let layout = Layout::from_size_align(cap, Self::ALIGN).unwrap();
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), layout);
        }
    }
}

impl io::Write for BridgeVec {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Raw 16-byte header representing `Ok(())` (Unit).
///
/// Use this to write directly to network streams without heap allocation.
///
/// DO NOT RETURN THIS IN AN FFI SITUATION
///
/// Layout:
/// - Magic:   0x7079726F ("pyro") (Little Endian: 6F 72 79 70)
/// - Len:     0
/// - Cap:     16 (Minimal Header Size)
/// - WireFmt: 1 (PROTOCOL_VERSION)
/// - Status:  0 (ValidData)
const UNIT_BYTES: [u8; 16] = [
    0x6F, 0x72, 0x79, 0x70, // 0x00: Magic
    0x00, 0x00, 0x00, 0x00, // 0x04: Len (0)
    0x10, 0x00, 0x00, 0x00, // 0x08: Cap (16)
    0x01, // 0x0C: WireFormat (1)
    0x00, // 0x0D: UserVer
    0x00, // 0x0E: ErrVer
    0x00, // 0x0F: Status (ValidData)
];
#[repr(C, align(16))]
struct AlignedHeader([u8; 16]);

// A static, aligned instance of the unit bytes
static UNIT_HEADER: AlignedHeader = AlignedHeader(UNIT_BYTES);


#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_bridge_vec_ok() {
        let vec = BridgeVec::ok();

        // Check Header correctness
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 16);
        assert_eq!(vec.status(), 0); // ValidData
        assert_eq!(vec.wire_format(), 1);

        // Check Memory Layout
        assert_eq!(vec.as_ptr() as usize % 16, 0, "Owned Ok must be aligned");

        // Ensure it is actually owned (drop shouldn't panic/segfault)
        drop(vec);
    }

    #[test]
    fn test_bridge_vec_ref_ok() {
        let vec_ref = BridgeVecRef::ok();

        assert_eq!(vec_ref.len(), 0);
        assert_eq!(vec_ref.status(), 0);
        assert_eq!(
            vec_ref.as_ptr() as usize % 16,
            0,
            "Static Ref must be aligned"
        );

        // Validate against raw bytes
        let slice = vec_ref.as_packet_slice();
        assert_eq!(slice, &UNIT_BYTES, "Ref should point to UNIT_BYTES content");
    }

    #[test]
    fn test_ok_interoperability() {
        let owned = BridgeVec::ok();
        let reference = BridgeVecRef::ok();

        // They should be byte-equivalent
        assert_eq!(owned.as_packet_slice(), reference.as_packet_slice());

        // But distinct addresses (Owned is on heap, Ref is in static text/data segment)
        assert_ne!(owned.as_ptr(), reference.as_ptr());
    }
}