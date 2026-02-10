use std::ops::Deref;

use tracing::trace;

use crate::error::ErrorKind;
use crate::header::{
    PyroData, PyroHeader, PyroHeaderMut, DataStatus, MAGIC_VAL, MutPyroData,
    PROTOCOL_VERSION,
};
use crate::view::PyroView;
use crate::{PyroError, PyroResult, PyroVec};

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

    fn parse_result<E>(
        self,
    ) -> Result<
        Result<<Self as Parser<BD, T>>::TypedWrapper, <Self as Parser<BD, E>>::TypedWrapper>,
        ReadError<BD>,
    >
    where
        Self: Parser<BD, E>,
        E: UserHeaderValues,
    {
        let status = <Self as Wrapper>::data(&self).status();
        let ok_code = <<Self as Parser<BD, T>>::HeaderValues as PyroHeaderValues>::OK_CODE;
        let err_code = <<Self as Parser<BD, T>>::HeaderValues as PyroHeaderValues>::ERR_CODE;

        trace!(
            ?status,
            ?ok_code,
            ?err_code,
            "Parser::parse_result checking status code"
        );

        match status {
            Ok(s) if s == ok_code => {
                trace!("Parser::parse_result matched OK_CODE, parsing as success type");
                let buf = <Self as Parser<BD, T>>::unchecked_parse(self)?;
                Ok(Ok(buf))
            }
            Ok(s) if s == err_code => {
                trace!("Parser::parse_result matched ERR_CODE, parsing as error type");
                let buf = <Self as Parser<BD, E>>::unchecked_parse(self)?;
                Ok(Err(buf))
            }
            _ => {
                trace!(
                    ?status,
                    "Parser::parse_result unknown status, returning InvalidCode"
                );
                Err(ReadError::InvalidCode(<Self as Wrapper>::into_data(self)))
            }
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
        bd.set_magic(MAGIC_VAL);
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
                let status =
                    <<Self as Writer<BD, E>>::HeaderValues as PyroHeaderValues>::ERR_CODE;
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

    type ViewParser<'a>: Parser<
            PyroView<'a>,
            T,
            HeaderValues = <Self as PyroFormat<T>>::HeaderValues,
            ParsedType = <Self as PyroFormat<T>>::ParsedType,
        >;

    fn view_parser<'a>(data: PyroView<'a>) -> Self::ViewParser<'a>;

    /// Parse a borrowed `PyroView` into a typed view wrapper.
    fn parse_view<'a>(
        vec: PyroView<'a>,
    ) -> Result<<Self::ViewParser<'a> as Parser<PyroView<'a>, T>>::TypedWrapper, PyroError>
    {
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
// PyroZeroCopyFormat — extension for zero-copy formats (rkyv, zerovec, …)
// =============================================================================

/// Extension trait for formats that support **zero-copy borrowed access**.
///
/// This adds:
/// - `ViewParser` — parsing a `PyroView<'a>` without taking ownership
/// - `parse_view` — the borrowed-view counterpart of `PyroFormat::parse`
///
/// Only formats where the parsed representation is a reference into the buffer
/// (like rkyv's `T::Archived`) should implement this. Owned formats like JSON
/// do not benefit from view parsing and should only implement `PyroFormat`.
pub trait PyroZeroCopyFormat<T>: PyroFormat<T>
where
    T: UserHeaderValues,
{
    type Receiver: Receiver<<Self::VecParser as Parser<PyroVec, T>>::ParsedType, T>;
    fn receiver() -> Self::Receiver;
}
