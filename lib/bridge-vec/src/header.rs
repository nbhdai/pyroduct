use std::convert::TryInto;
use std::ops::{Deref, DerefMut};
use std::ptr;
use thiserror::Error;

pub(crate) const MAGIC_VAL: u32 = 0x7079726F; // "pyro"
pub const PROTOCOL_VERSION: u8 = 1;

/// Status codes located at Offset 0x0F in the header.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    ValidData = 0,
    UserError = 1,
    CodeError = 3,

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

/// Errors that occur when parsing or validating a BridgeVec header.
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
    #[error("invalid capacity must be at least 16")]
    InvalidCapacity,
    #[error("length header exceeds capacity")]
    LengthExceedsCapacity,
    #[error("unsupported wire format")]
    UnsupportedWireFormat,
}

// --- Sealing Module ---
mod private {
    use crate::{BridgeVec, view::BridgeView};

    pub trait Sealed {}
    // The primitive array is the base sealed type.
    impl Sealed for [u8; 16] {}
    impl Sealed for BridgeVec {}
    impl Sealed for BridgeView<'_> {}
}

/// A specialized parser for the 16-byte BridgeVec header.
#[derive(Debug, Clone, Copy)]
pub struct BridgeParser;

impl BridgeParser {
    pub const ALIGN: usize = 16;
    pub const HEADER_SIZE: usize = 16;
    
    // Offsets
    pub const OFFSET_MAGIC: usize = 0;
    pub const OFFSET_LEN: usize = 4;
    pub const OFFSET_CAP: usize = 8;
    pub const OFFSET_WIRE: usize = 12;
    pub const OFFSET_USER_VER: usize = 13;
    pub const OFFSET_ERR_VER: usize = 14;
    pub const OFFSET_STATUS: usize = 15;

    pub fn check_strict(slice: &[u8]) -> Result<(), ParseError> {
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

        let cap = Self::read_u32(slice, Self::OFFSET_CAP);
        if (cap as usize) < Self::HEADER_SIZE {
            return Err(ParseError::InvalidCapacity);
        }

        let len = Self::read_u32(slice, Self::OFFSET_LEN);
        if (len as usize).saturating_add(Self::HEADER_SIZE) > (cap as usize) {
            return Err(ParseError::LengthExceedsCapacity);
        }

        let wire = slice[Self::OFFSET_WIRE];
        if wire != PROTOCOL_VERSION {
            return Err(ParseError::UnsupportedWireFormat);
        }

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

        let cap = Self::read_u32(slice, Self::OFFSET_CAP);
        if slice.len() < cap as usize {
            return Err(ParseError::SliceTooSmall);
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

    pub unsafe fn check_strict_raw(ptr: *const u8) -> Result<(), ParseError> {
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

        let cap = unsafe { ptr::read(ptr.add(Self::OFFSET_CAP) as *const u32) };
        if (cap as usize) < Self::HEADER_SIZE {
            return Err(ParseError::InvalidCapacity);
        }

        let len = unsafe { ptr::read(ptr.add(Self::OFFSET_LEN) as *const u32) };
        if (len as usize).saturating_add(Self::HEADER_SIZE) > (cap as usize) {
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
        let bytes = slice[offset..offset+4].try_into().unwrap();
        u32::from_le_bytes(bytes)
    }
}

// --- Trait Definitions ---

pub trait BridgeHeader: private::Sealed {
    fn magic(&self) -> u32;
    fn header_len(&self) -> u32;
    fn header_capacity(&self) -> u32;
    fn wire_format(&self) -> u8;
    fn version(&self) -> u8;
    fn error_version(&self) -> u8;
    fn status_u8(&self) -> u8;

    fn status(&self) -> Result<DataStatus, u8>;
    fn is_ok(&self) -> bool;
    fn is_user_err(&self) -> bool;
    fn is_bridge_err(&self) -> bool;
}

pub(crate) trait BridgeHeaderMut: private::Sealed {
    fn set_magic(&mut self, magic: u32);
    fn set_len(&mut self, len: u32);
    fn set_capacity(&mut self, cap: u32);
    fn set_wire_format(&mut self, wire_fmt: u8);
    fn set_version(&mut self, version: u8);
    fn set_error_version(&mut self, err_version: u8);

    #[inline]
    fn set_status(&mut self, status: DataStatus) {
        self.set_status_u8(status as u8);
    }
    fn set_status_u8(&mut self, status: u8);
    fn init(&mut self);
}

/// Defines a type that contains a bridge header.
/// This is the primary interface for read-only access.
pub trait BridgeData: private::Sealed + Deref<Target = [u8]> {
    fn header(&self) -> &[u8; 16];
}

/// Defines a type that means we own this data.
/// 
/// Note: The signature for `header_mut` requires `&mut self`.
pub trait OwnedBridgeData: BridgeData + DerefMut<Target = [u8]> {
    fn header_mut(&mut self) -> &mut [u8; 16];
}

/// Defines a type that means we have a mutable reference to this data.
/// We're allowed to mutate the buffer but we cannot deallocate it.
/// 
/// Note: The signature for `header_mut` requires `&mut self`.
pub trait MutBridgeData: BridgeData + DerefMut<Target = [u8]> {
    fn header_mut(&mut self) -> &mut [u8; 16];
}

impl<T: OwnedBridgeData> MutBridgeData for T {
    fn header_mut(&mut self) -> &mut [u8; 16] {
        self.header_mut()
    }
}

// --- Blanket Logic Implementations ---

impl<T: BridgeData> BridgeHeader for T {
    #[inline]
    fn magic(&self) -> u32 {
        let h = self.header();
        let bytes = h[BridgeParser::OFFSET_MAGIC..BridgeParser::OFFSET_MAGIC + 4].try_into().unwrap();
        u32::from_le_bytes(bytes)
    }

    #[inline]
    fn header_len(&self) -> u32 {
        let h = self.header();
        let bytes = h[BridgeParser::OFFSET_LEN..BridgeParser::OFFSET_LEN + 4].try_into().unwrap();
        u32::from_le_bytes(bytes)
    }

    #[inline]
    fn header_capacity(&self) -> u32 {
        let h = self.header();
        let bytes = h[BridgeParser::OFFSET_CAP..BridgeParser::OFFSET_CAP + 4].try_into().unwrap();
        u32::from_le_bytes(bytes)
    }

    #[inline]
    fn wire_format(&self) -> u8 {
        self.header()[BridgeParser::OFFSET_WIRE]
    }

    #[inline]
    fn version(&self) -> u8 {
        self.header()[BridgeParser::OFFSET_USER_VER]
    }

    #[inline]
    fn error_version(&self) -> u8 {
        self.header()[BridgeParser::OFFSET_ERR_VER]
    }

    #[inline]
    fn status_u8(&self) -> u8 {
        self.header()[BridgeParser::OFFSET_STATUS]
    }

    fn status(&self) -> Result<DataStatus, u8> {
        let status_byte = self.status_u8();
        match status_byte {
            0 => Ok(DataStatus::ValidData),
            1 => Ok(DataStatus::UserError),
            3 => Ok(DataStatus::CodeError),
            
            100 => Ok(DataStatus::LocalSerialization),
            101 => Ok(DataStatus::LocalDeserialization),
            102 => Ok(DataStatus::LocalValidation),
            103 => Ok(DataStatus::LocalTransport),
            104 => Ok(DataStatus::LocalIo),
            105 => Ok(DataStatus::LocalUtf8),
            106 => Ok(DataStatus::LocalInvalidHeader),
            107 => Ok(DataStatus::LocalLayoutError),
            108 => Ok(DataStatus::LocalUnexpectedEof),

            150 => Ok(DataStatus::RemoteSerialization),
            151 => Ok(DataStatus::RemoteDeserialization),
            152 => Ok(DataStatus::RemoteValidation),
            153 => Ok(DataStatus::RemoteTransport),
            154 => Ok(DataStatus::RemoteIo),
            155 => Ok(DataStatus::RemoteUtf8),
            156 => Ok(DataStatus::RemoteInvalidHeader),
            157 => Ok(DataStatus::RemoteLayoutError),
            158 => Ok(DataStatus::RemoteUnexpectedEof),

            other => Err(other),
        }
    }

    fn is_ok(&self) -> bool {
        self.status_u8() == 0
    }

    fn is_user_err(&self) -> bool {
        self.status_u8() == 1
    }

    fn is_bridge_err(&self) -> bool {
        let s = self.status_u8();
        s != 0 && s != 1
    }
}

impl<T: MutBridgeData> BridgeHeaderMut for T {
    #[inline]
    fn set_magic(&mut self, magic: u32) {
        let bytes = magic.to_le_bytes();
        let h = self.header_mut();
        h[BridgeParser::OFFSET_MAGIC..BridgeParser::OFFSET_MAGIC + 4].copy_from_slice(&bytes);
    }

    #[inline]
    fn set_len(&mut self, len: u32) {
        let bytes = len.to_le_bytes();
        let h = self.header_mut();
        h[BridgeParser::OFFSET_LEN..BridgeParser::OFFSET_LEN + 4].copy_from_slice(&bytes);
    }

    #[inline]
    fn set_capacity(&mut self, cap: u32) {
        let bytes = cap.to_le_bytes();
        let h = self.header_mut();
        h[BridgeParser::OFFSET_CAP..BridgeParser::OFFSET_CAP + 4].copy_from_slice(&bytes);
    }

    #[inline]
    fn set_wire_format(&mut self, wire_fmt: u8) {
        self.header_mut()[BridgeParser::OFFSET_WIRE] = wire_fmt;
    }

    #[inline]
    fn set_version(&mut self, version: u8) {
        self.header_mut()[BridgeParser::OFFSET_USER_VER] = version;
    }

    #[inline]
    fn set_error_version(&mut self, err_version: u8) {
        self.header_mut()[BridgeParser::OFFSET_ERR_VER] = err_version;
    }

    #[inline]
    fn set_status_u8(&mut self, status: u8) {
        self.header_mut()[BridgeParser::OFFSET_STATUS] = status;
    }

    fn init(&mut self) {
        self.set_magic(MAGIC_VAL);
        self.set_len(0);
        self.set_capacity(16);
        self.set_wire_format(PROTOCOL_VERSION);
        self.set_version(0);
        self.set_error_version(0);
        self.set_status(DataStatus::ValidData);
    }
}

pub(crate) const UNIT_BYTES: [u8; 16] = [
    0x6F, 0x72, 0x79, 0x70, // Magic
    0x00, 0x00, 0x00, 0x00, // Len (0)
    0x10, 0x00, 0x00, 0x00, // Cap (16)
    0x01,                   // WireFormat (1)
    0x00,                   // UserVer
    0x00,                   // ErrVer
    0x00,                   // Status (ValidData)
];

#[repr(C, align(16))]
pub struct StaticHeader(pub(crate) [u8; 16]);

pub static UNIT_HEADER: StaticHeader = StaticHeader(UNIT_BYTES);