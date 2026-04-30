use std::ops::Deref;

use tracing::trace;

use crate::error::ErrorKind;
use crate::format::{
    PyroVec,
    header::{DataStatus, MutPyroData, PROTOCOL_VERSION, PyroData, PyroHeader, PyroHeaderMut},
    PyroView,
};
use crate::{CapturedError, PyroError, PyroResult};

// =============================================================================
// Foundational wrapper / typed-wrapper traits (unchanged)
// =============================================================================

pub trait Wrapper {
    type Wrapping;
    fn data(&self) -> &Self::Wrapping;
    fn into_data(self) -> Self::Wrapping;
}

pub trait MutWrapper: Wrapper {
    fn data_mut(&mut self) -> &mut Self::Wrapping;
}

pub trait TypedWrapper<T>: Wrapper + Deref<Target = T> {}

pub trait HasReceiver<P, T>: TypedWrapper<P> {
    type Receiver;
    fn receiver(&self) -> Self::Receiver;
}

pub trait Receiver<P, T> {
    fn receive<D>(&mut self, data: &D) -> PyroResult<T>
    where
        D: Deref<Target = P>;
}

// =============================================================================
// Header values
// =============================================================================

pub trait PyroHeaderValues {
    const OK_CODE: DataStatus;
    const ERR_CODE: DataStatus;
}

pub trait UserHeaderValues {
    const VERSION: u8 = 0;
}

// =============================================================================
// ReadError (unchanged)
// =============================================================================

pub enum ReadError<T> {
    InvalidCode(T),
    Pyro(PyroError),
}

impl<T> From<PyroError> for ReadError<T> {
    fn from(value: PyroError) -> Self {
        ReadError::Pyro(value)
    }
}

// =============================================================================
// Parser trait (unchanged)
// =============================================================================

pub trait Parser<BD: PyroData, T>: Wrapper<Wrapping = BD> + Sized
where
    T: UserHeaderValues,
{
    type HeaderValues: PyroHeaderValues;
    type ParsedType;
    type TypedWrapper: TypedWrapper<Self::ParsedType>;

    fn unchecked_parse(self) -> PyroResult<Self::TypedWrapper>;

    fn parse(self) -> Result<Self::TypedWrapper, ReadError<BD>> {
        let status = <Self as Wrapper>::data(&self).status();
        let expected = Self::HeaderValues::OK_CODE;

        trace!(?status, ?expected, "Parser::parse checking status code");

        if status == Ok(expected) {
            trace!("Parser::parse status OK, proceeding with unchecked_parse");
            let buf = self.unchecked_parse()?;
            trace!("Parser::parse unchecked_parse succeeded");
            Ok(buf)
        } else {
            trace!(
                ?status,
                "Parser::parse status mismatch, returning InvalidCode"
            );
            Err(ReadError::InvalidCode(self.into_data()))
        }
    }
}

// =============================================================================
// Writer trait (unchanged)
// =============================================================================

pub trait Writer<BD: MutPyroData, T>: Wrapper<Wrapping = BD> + MutWrapper + Sized
where
    T: UserHeaderValues,
{
    type HeaderValues: PyroHeaderValues;

    fn write_raw(&mut self, value: &T) -> PyroResult<usize>;

    fn update_header(&mut self, len: u32, status: DataStatus) {
        trace!(len, ?status, "Writer::update_header setting header fields");
        let bd = self.data_mut();
        bd.set_wire_format(PROTOCOL_VERSION);
        bd.set_status(status);
        trace!("Writer::update_header completed");
    }

    fn write(mut self, value: &T) -> PyroResult<BD> {
        trace!("Writer::write starting serialization");
        let len = self.write_raw(value)?;
        trace!(
            len,
            "Writer::write raw write completed, updating header with OK_CODE"
        );
        self.update_header(
            len as u32,
            <Self::HeaderValues as PyroHeaderValues>::OK_CODE,
        );
        trace!("Writer::write completed successfully");
        Ok(self.into_data())
    }

    fn write_result<E>(mut self, value: Result<&T, &E>) -> PyroResult<BD>
    where
        Self: Writer<BD, E>,
        E: UserHeaderValues,
    {
        match value {
            Ok(val) => {
                trace!("Writer::write_result serializing Ok variant");
                let len = <Self as Writer<BD, T>>::write_raw(&mut self, val)?;
                let status = <<Self as Writer<BD, T>>::HeaderValues as PyroHeaderValues>::OK_CODE;
                trace!(len, ?status, "Writer::write_result Ok variant written");
                <Self as Writer<BD, T>>::update_header(&mut self, len as u32, status);
                Ok(self.into_data())
            }
            Err(err) => {
                trace!("Writer::write_result serializing Err variant");
                let len = <Self as Writer<BD, E>>::write_raw(&mut self, err)?;
                let status = <<Self as Writer<BD, E>>::HeaderValues as PyroHeaderValues>::ERR_CODE;
                trace!(len, ?status, "Writer::write_result Err variant written");
                <Self as Writer<BD, E>>::update_header(&mut self, len as u32, status);
                Ok(self.into_data())
            }
        }
    }
}

// =============================================================================
// PyroFormat — base trait for ALL formats (owned deserialization)
// =============================================================================

/// Base format trait implemented by every pyro format (JSON, rkyv, zerovec, …).
///
/// Provides:
/// - Serialization (`ship`) via `VecWriter`
/// - Owned-buffer parsing (`parse`) via `VecParser`
/// - A `Receiver` to extract `T` from the parsed representation
///
/// The `ParsedType` is format-dependent:
/// - For owned formats (JSON): `ParsedType = T`
/// - For zero-copy formats (rkyv): `ParsedType = T::Archived`
///
/// Zero-copy formats additionally implement [`PyroZeroCopyFormat`] to gain
/// borrowed-view parsing.
pub trait PyroFormat<T>: Sized
where
    T: UserHeaderValues,
{
    /// The wire-format byte written to header offset `0x0C`.
    /// Used by fallback pyros to dispatch on incoming data.
    const WIRE_FORMAT: u8;

    type HeaderValues: PyroHeaderValues;

    /// The type you get when you `Deref` the parsed wrapper.
    /// - Owned formats: `T`
    /// - Zero-copy formats: `T::Archived` (or similar)
    type ParsedType;

    type VecParser: Parser<PyroVec, T, HeaderValues = Self::HeaderValues, ParsedType = Self::ParsedType>;
    type VecWriter: Writer<PyroVec, T, HeaderValues = Self::HeaderValues>;

    fn new() -> Self;
    fn vec_parser(data: PyroVec) -> Self::VecParser;
    fn new_writer(data: PyroVec) -> Self::VecWriter;

    /// Serialize `T` into a new `PyroVec`.
    fn ship(input: &T) -> Result<PyroVec, PyroError> {
        trace!(
            type_name = std::any::type_name::<T>(),
            "PyroFormat::ship starting serialization"
        );
        let vec = PyroVec::ok();
        let writer = Self::new_writer(vec);
        let result = writer.write(input);
        match &result {
            Ok(v) => trace!(
                len = v.len(),
                capacity = v.capacity(),
                "PyroFormat::ship succeeded"
            ),
            Err(e) => trace!(error = ?e, "PyroFormat::ship failed"),
        }
        result
    }

    /// Parse an owned `PyroVec` into a typed wrapper.
    fn parse(
        vec: PyroVec,
    ) -> Result<<Self::VecParser as Parser<PyroVec, T>>::TypedWrapper, PyroError> {
        trace!(
            type_name = std::any::type_name::<T>(),
            len = vec.len(),
            status = ?vec.status(),
            "PyroFormat::parse starting"
        );
        match Self::vec_parser(vec).parse() {
            Ok(typed) => {
                trace!("PyroFormat::parse succeeded");
                Ok(typed)
            }
            Err(e) => {
                let err = match e {
                    ReadError::Pyro(be) => be,
                    ReadError::InvalidCode(data) => match data.parse_as_error() {
                        Ok(()) => PyroError::local(ErrorKind::InvalidHeader),
                        Err(be) => be,
                    },
                };
                Err(err)
            }
        }
    }

    type ViewParser: Parser<
            PyroView,
            T,
            HeaderValues = <Self as PyroFormat<T>>::HeaderValues,
            ParsedType = <Self as PyroFormat<T>>::ParsedType,
        >;

    fn view_parser(data: PyroView) -> Self::ViewParser;

    /// Parse a borrowed `PyroView` into a typed view wrapper.
    fn parse_view(
        vec: PyroView,
    ) -> Result<<Self::ViewParser as Parser<PyroView, T>>::TypedWrapper, PyroError> {
        trace!(
            type_name = std::any::type_name::<T>(),
            len = vec.len(),
            status = ?vec.status(),
            "PyroZeroCopyFormat::parse_view starting"
        );
        match Self::view_parser(vec).parse() {
            Ok(typed) => {
                trace!("PyroZeroCopyFormat::parse_view succeeded");
                Ok(typed)
            }
            Err(e) => {
                let err = match e {
                    ReadError::Pyro(be) => be,
                    ReadError::InvalidCode(data) => match data.parse_as_error() {
                        Ok(()) => PyroError::local(ErrorKind::InvalidHeader),
                        Err(be) => be,
                    },
                };
                Err(err)
            }
        }
    }
}

// =============================================================================
// SpecWire
// =============================================================================

/// Trait for types that can be encoded as a specification on the wire.
pub trait SpecWire: Sized {
    fn to_wire(&self) -> PyroResult<PyroVec>;
    fn parse_wire(view: PyroView) -> PyroResult<Self>;
}

impl SpecWire for pyro_spec::InterfaceSpec<'_> {
    fn to_wire(&self) -> PyroResult<PyroVec> {
        let bytes = serde_json::to_vec(self)
            .map_err(|e| PyroError::serialization(CapturedError::new("Unable to serialize interface spec").with_source(e)))?;

        let mut vec = PyroVec::with_capacity(bytes.len());
        vec.extend_from_slice(&bytes);
        vec.set_status(crate::format::header::DataStatus::InterfaceSchema);
        Ok(vec)
    }

    fn parse_wire(view: PyroView) -> PyroResult<Self> {
        if let Ok(DataStatus::InterfaceSchema) = view.status() {
            let interface = serde_json::from_slice(&view).map_err(|e| PyroError::serialization(CapturedError::new("Unable to deserialize interface spec").with_source(e)))?;
            Ok(interface)
        } else {
            Err(PyroError::Header(super::ParseError::UnknownStatus(view.status_u8())))
        }
    }
}
