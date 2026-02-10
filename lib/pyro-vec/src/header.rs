use crate::error::ErrorKind;
use crate::{PyroError, CapturedError};
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};
use std::ptr;
use thiserror::Error;

pub const MAGIC_VAL: u32 = 0x7079726F; // "pyro"
pub const PROTOCOL_VERSION: u8 = 1;

macro_rules! define_data_status {
    ($( $name:ident = $val:expr ),* $(,)?) => {
        /// Status codes located at Offset 0x0F in the header.
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum DataStatus {
            $( $name = $val, )*
        }

        impl DataStatus {
            pub fn parse(i: u8) -> Result<DataStatus, u8> {
                match i {
                    $( $val => Ok(DataStatus::$name), )*
                    other => Err(other),
                }
            }
        }
    };
}

define_data_status! {
    Empty = 0,
    CodeError = 1,
    RkyvValid = 2,
    RkyvError = 3,
    JsonValid = 4,
    JsonError = 5,

    // A ffi pyro was unable to parse a header indicating a critical error.
    PyroFfiFail        = 99,
    // --- Local Errors (100-149) ---
    LocalSerialization   = 100,
    LocalDeserialization = 101,
    LocalValidation      = 102,
    LocalTransport       = 103,
    LocalIo              = 104,
    LocalUtf8            = 105,
    LocalInvalidHeader   = 106,
    LocalLayoutError     = 107,
    LocalUnexpectedEof   = 108,

    // --- Remote Errors (150-199) ---
    RemoteSerialization   = 150,
    RemoteDeserialization = 151,
    RemoteValidation      = 152,
    RemoteTransport       = 153,
    RemoteIo              = 154,
    RemoteUtf8            = 155,
    RemoteInvalidHeader   = 156,
    RemoteLayoutError     = 157,
    RemoteUnexpectedEof   = 158,
}

/// Errors that occur when parsing or validating a PyroVec header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("pointer is null")]
    NullPointer,
    #[error("The data indicated by this header does not fit in the slice")]
    SliceTooSmall,
    #[error("pointer is misaligned")]
    MisalignedPointer,
    #[error("invalid magic header")]
    InvalidMagic,
    #[error("length header exceeds capacity")]
    LengthExceedsCapacity,
    #[error("unsupported wire format")]
    UnsupportedWireFormat,
}

// --- Sealing Module ---
mod private {
    use rkyv::Archive;

    use crate::{
        PyroBuf, PyroVec, TypedPyroView, TypedBuf,
        view::{PyroMutView, PyroView},
    };

    pub trait Sealed {}
    // The primitive array is the base sealed type.
    impl Sealed for [u8; 16] {}
    impl Sealed for PyroVec {}
    impl Sealed for PyroBuf {}
    impl<T: Archive> Sealed for TypedBuf<T> {}
    impl Sealed for PyroView<'_> {}
    impl<T: Archive> Sealed for TypedPyroView<'_, T> {}
    impl Sealed for PyroMutView<'_> {}
}

/// A specialized parser for the 16-byte PyroVec header.
#[derive(Debug, Clone, Copy)]
pub struct PyroParser;

impl PyroParser {
    pub const ALIGN: usize = 16;
    pub const HEADER_SIZE: usize = 16;

    // Offsets
    pub const OFFSET_MAGIC: usize = 0;
    pub const OFFSET_LEN: usize = 4;
    pub const OFFSET_WIRE: usize = 8;
    pub const OFFSET_STATUS: usize = 9;
    pub const OFFSET_ERR_VER: usize = 10;
    pub const OFFSET_USER_VER: usize = 11;

    pub fn check_strict(slice: &[u8]) -> Result<(), ParseError> {
        Self::check(slice)?;

        let len = Self::read_u32(slice, Self::OFFSET_LEN);
        if len as usize > slice.len() {
            return Err(ParseError::LengthExceedsCapacity);
        }

        let wire = slice[Self::OFFSET_WIRE];
        if wire != PROTOCOL_VERSION {
            return Err(ParseError::UnsupportedWireFormat);
        }
        // Todo: check status codes aren't an error

        Ok(())
    }

    pub fn check(slice: &[u8]) -> Result<(), ParseError> {
        if slice.len() < Self::HEADER_SIZE {
            return Err(ParseError::SliceTooSmall);
        }
        if (slice.as_ptr() as usize) % Self::ALIGN != 0 {
            return Err(ParseError::MisalignedPointer);
        }

        let magic = Self::read_u32(slice, Self::OFFSET_MAGIC);
        if magic != MAGIC_VAL {
            return Err(ParseError::InvalidMagic);
        }

        let len = Self::read_u32(slice, Self::OFFSET_LEN);
        if len as usize > slice.len() {
            return Err(ParseError::LengthExceedsCapacity);
        }

        Ok(())
    }

    pub unsafe fn check_raw(ptr: *const u8) -> Result<(), ParseError> {
        if ptr.is_null() {
            return Err(ParseError::NullPointer);
        }
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err(ParseError::MisalignedPointer);
        }

        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != MAGIC_VAL {
            return Err(ParseError::InvalidMagic);
        }

        Ok(())
    }

    pub unsafe fn check_strict_raw(ptr: *const u8, capacity: usize) -> Result<(), ParseError> {
        if ptr.is_null() {
            return Err(ParseError::NullPointer);
        }
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err(ParseError::MisalignedPointer);
        }

        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != MAGIC_VAL {
            return Err(ParseError::InvalidMagic);
        }

        let len = unsafe { ptr::read(ptr.add(Self::OFFSET_LEN) as *const u32) };
        if len as usize > capacity {
            return Err(ParseError::LengthExceedsCapacity);
        }

        let wire = unsafe { ptr::read(ptr.add(Self::OFFSET_WIRE) as *const u8) };
        if wire != PROTOCOL_VERSION {
            return Err(ParseError::UnsupportedWireFormat);
        }

        Ok(())
    }

    #[inline]
    fn read_u32(slice: &[u8], offset: usize) -> u32 {
        let bytes = slice[offset..offset + 4].try_into().unwrap();
        u32::from_le_bytes(bytes)
    }
}

// --- Trait Definitions ---

pub trait PyroHeader: private::Sealed {
    fn magic(&self) -> u32;
    fn header_len(&self) -> u32;
    fn wire_format(&self) -> u8;
    fn version(&self) -> u8;
    fn error_version(&self) -> u8;
    fn status_u8(&self) -> u8;

    fn status(&self) -> Result<DataStatus, u8>;
    fn is_ok(&self) -> bool;
    fn is_user_err(&self) -> bool;
    fn is_pyro_err(&self) -> bool;
}

pub trait PyroHeaderMut: private::Sealed {
    fn set_magic(&mut self, magic: u32);
    unsafe fn set_len(&mut self, len: u32);
    fn set_wire_format(&mut self, wire_fmt: u8);
    fn set_version(&mut self, version: u8);
    fn set_error_version(&mut self, err_version: u8);

    #[inline]
    fn set_status(&mut self, status: DataStatus) {
        self.set_status_u8(status as u8);
    }
    fn set_status_u8(&mut self, status: u8);
}

/// Defines a type that contains a pyro header.
/// This is the primary interface for read-only access.
pub trait PyroData: private::Sealed + Deref<Target = [u8]> + Sized {
    fn header(&self) -> &[u8; 16];
    fn capacity(&self) -> usize;

    /// Consumes the PyroVec and converts it into the appropriate PyroError
    /// based on the Status header.
    ///
    /// If the status implies a payload (e.g., CodeError/Panic), this attempts
    /// to deserialize the payload as a JSON `CapturedError`.
    /// Parse this PyroVec as an error based on its status code.
    fn parse_as_error(&self) -> Result<(), PyroError>
    where
        Self: PyroData,
    {
        Err(
            match DataStatus::parse(self.header()[PyroParser::OFFSET_STATUS]) {
                Ok(DataStatus::Empty)
                | Ok(DataStatus::RkyvValid)
                | Ok(DataStatus::RkyvError)
                | Ok(DataStatus::JsonValid)
                | Ok(DataStatus::JsonError) => return Ok(()),
                Err(_) => return Ok(()),

                // Remote errors (150-156, 3)
                Ok(DataStatus::CodeError) => PyroError::CodePanic(self.extract_captured_error()),
                Ok(DataStatus::RemoteSerialization) => {
                    PyroError::remote(ErrorKind::Serialization(self.extract_captured_error()))
                }
                Ok(DataStatus::RemoteValidation) => {
                    PyroError::remote(ErrorKind::Serialization(self.extract_captured_error()))
                }
                Ok(DataStatus::RemoteDeserialization) => {
                    PyroError::remote(ErrorKind::Deserialization(self.extract_captured_error()))
                }
                Ok(DataStatus::RemoteTransport) => {
                    PyroError::remote(ErrorKind::Transport(self.extract_captured_error()))
                }
                Ok(DataStatus::RemoteIo) => PyroError::remote_io(self.extract_captured_error()),
                Ok(DataStatus::RemoteUtf8) => {
                    PyroError::remote_utf8(self.extract_captured_error())
                }
                Ok(DataStatus::RemoteUnexpectedEof) => {
                    PyroError::remote(ErrorKind::InvalidHeader)
                }
                Ok(DataStatus::RemoteInvalidHeader) => {
                    PyroError::remote(ErrorKind::InvalidHeader)
                }
                Ok(DataStatus::RemoteLayoutError) => PyroError::remote(ErrorKind::LayoutError),

                Ok(DataStatus::PyroFfiFail) => {
                    PyroError::HeaderFfi(self.extract_captured_error())
                }
                // Local errors (100-109) - shouldn't normally appear in received data
                // but handle them for completeness
                Ok(DataStatus::LocalSerialization) => {
                    PyroError::local(ErrorKind::Serialization(self.extract_captured_error()))
                }
                Ok(DataStatus::LocalValidation) => {
                    PyroError::local(ErrorKind::Serialization(self.extract_captured_error()))
                }
                Ok(DataStatus::LocalDeserialization) => {
                    PyroError::local(ErrorKind::Deserialization(self.extract_captured_error()))
                }
                Ok(DataStatus::LocalTransport) => {
                    PyroError::local(ErrorKind::Transport(self.extract_captured_error()))
                }
                Ok(DataStatus::LocalIo) => {
                    PyroError::local(ErrorKind::Transport(self.extract_captured_error()))
                }
                Ok(DataStatus::LocalUtf8) => {
                    PyroError::local(ErrorKind::Transport(self.extract_captured_error()))
                }
                Ok(DataStatus::LocalInvalidHeader) => PyroError::local(ErrorKind::InvalidHeader),
                Ok(DataStatus::LocalLayoutError) => PyroError::local(ErrorKind::LayoutError),
                Ok(DataStatus::LocalUnexpectedEof) => PyroError::local(ErrorKind::UnexpectedEof),
            },
        )
    }

    /// Helper to deserialize a CapturedError from the payload (JSON).
    /// Falls back to a generic error if JSON deserialization fails.
    fn extract_captured_error(&self) -> Box<CapturedError> {
        if let Ok(captured) = serde_json::from_slice::<CapturedError>(&*self) {
            Box::new(captured)
        } else {
            Box::new(CapturedError {
                message: String::from_utf8_lossy(&*self).to_string(),
                file: "unknown".to_string(),
                line: 0,
                column: 0,
                error: Some("Failed to deserialize error details".into()),
                context: None,
                stack_trace: None,
                library: None,
            })
        }
    }
}

pub enum ReadError<T> {
    InvalidCode(T),
    Pyro(PyroError),
}

impl<T> From<PyroError> for ReadError<T> {
    fn from(value: PyroError) -> Self {
        ReadError::Pyro(value)
    }
}

/// Defines a type that means we own this data.
///
/// Note: The signature for `header_mut` requires `&mut self`.
pub trait OwnedPyroData: PyroData + DerefMut<Target = [u8]> {
    fn header_mut(&mut self) -> &mut [u8; 16];
}

/// Defines a type that means we have a mutable reference to this data.
/// We're allowed to mutate the buffer but we cannot deallocate it.
///
/// Note: The signature for `header_mut` requires `&mut self`.
pub trait MutPyroData: PyroData + DerefMut<Target = [u8]> {
    fn header_mut(&mut self) -> &mut [u8; 16];
}

impl<T: OwnedPyroData> MutPyroData for T {
    fn header_mut(&mut self) -> &mut [u8; 16] {
        self.header_mut()
    }
}

// --- Blanket Logic Implementations ---

impl<T: PyroData> PyroHeader for T {
    #[inline]
    fn magic(&self) -> u32 {
        let h = self.header();
        let bytes = h[PyroParser::OFFSET_MAGIC..PyroParser::OFFSET_MAGIC + 4]
            .try_into()
            .unwrap();
        u32::from_le_bytes(bytes)
    }

    #[inline]
    fn header_len(&self) -> u32 {
        let h = self.header();
        let bytes = h[PyroParser::OFFSET_LEN..PyroParser::OFFSET_LEN + 4]
            .try_into()
            .unwrap();
        u32::from_le_bytes(bytes)
    }

    #[inline]
    fn wire_format(&self) -> u8 {
        self.header()[PyroParser::OFFSET_WIRE]
    }

    #[inline]
    fn version(&self) -> u8 {
        self.header()[PyroParser::OFFSET_USER_VER]
    }

    #[inline]
    fn error_version(&self) -> u8 {
        self.header()[PyroParser::OFFSET_ERR_VER]
    }

    #[inline]
    fn status_u8(&self) -> u8 {
        self.header()[PyroParser::OFFSET_STATUS]
    }

    fn status(&self) -> Result<DataStatus, u8> {
        DataStatus::parse(self.status_u8())
    }

    fn is_ok(&self) -> bool {
        self.status_u8() == 0
    }

    fn is_user_err(&self) -> bool {
        self.status_u8() == 1
    }

    fn is_pyro_err(&self) -> bool {
        let s = self.status_u8();
        s != 0 && s != 1
    }
}

impl<T: MutPyroData> PyroHeaderMut for T {
    #[inline]
    fn set_magic(&mut self, magic: u32) {
        let bytes = magic.to_le_bytes();
        let h = self.header_mut();
        h[PyroParser::OFFSET_MAGIC..PyroParser::OFFSET_MAGIC + 4].copy_from_slice(&bytes);
    }

    #[inline]
    unsafe fn set_len(&mut self, len: u32) {
        let bytes = len.to_le_bytes();
        let h = self.header_mut();
        h[PyroParser::OFFSET_LEN..PyroParser::OFFSET_LEN + 4].copy_from_slice(&bytes);
    }

    #[inline]
    fn set_wire_format(&mut self, wire_fmt: u8) {
        self.header_mut()[PyroParser::OFFSET_WIRE] = wire_fmt;
    }

    #[inline]
    fn set_version(&mut self, version: u8) {
        self.header_mut()[PyroParser::OFFSET_USER_VER] = version;
    }

    #[inline]
    fn set_error_version(&mut self, err_version: u8) {
        self.header_mut()[PyroParser::OFFSET_ERR_VER] = err_version;
    }

    #[inline]
    fn set_status_u8(&mut self, status: u8) {
        self.header_mut()[PyroParser::OFFSET_STATUS] = status;
    }
}

pub const UNIT_BYTES: [u8; 16] = [
    0x6F, 0x72, 0x79, 0x70, // Magic
    0x00, 0x00, 0x00, 0x00, // Len (0)
    0x01, // WireFormat (1)
    0x00, // Status (ValidData)
    0x00, // UserVer
    0x00, // ErrVer
    0x00, 0x00, 0x00, 0x00, // Padding (0)
];

#[repr(C, align(16))]
pub struct StaticHeader(pub(crate) [u8; 16]);

pub static UNIT_HEADER: StaticHeader = StaticHeader(UNIT_BYTES);
