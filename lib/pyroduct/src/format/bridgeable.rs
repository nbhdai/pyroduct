//! Bridgeable trait and BridgeableResult extension.

use tracing::{debug, trace, warn};

use crate::PyroError;
use crate::format::format::{
    Parser, PyroFormat, PyroHeaderValues, PyroZeroCopyFormat, UserHeaderValues,
};
use crate::format::header::{PyroData, PyroHeader, PyroHeaderMut};
use crate::format::{ParseError, PyroRef, PyroVec, PyroView};
// =============================================================================
// Bridgeable — default-format convenience (every format)
// =============================================================================

/// A type that has a **default pyro format**.
///
/// This is the main entry point for most users. The `#[bridgeable]` macro
/// generates this impl with `type Format = Rkyv<Self>`.
///
/// For explicit format control, use [`Pyro<T, F>`](crate::pyro::Pyro)
/// instead.
pub trait Bridgeable: UserHeaderValues + Sized {
    type Format: PyroFormat<Self>;

    fn format() -> Self::Format {
        <Self::Format as PyroFormat<Self>>::new()
    }

    /// Serialize into a `PyroVec` using the default format.
    fn ship(&self) -> Result<PyroVec, PyroError> {
        trace!(
            type_name = std::any::type_name::<Self>(),
            version = Self::VERSION,
            "shipping value"
        );
        let result = <Self as Bridgeable>::Format::ship(self);
        match &result {
            Ok(vec) => debug!(
                type_name = std::any::type_name::<Self>(),
                len = vec.len(),
                capacity = vec.capacity(),
                "ship completed successfully"
            ),
            Err(e) => warn!(
                type_name = std::any::type_name::<Self>(),
                error = %e,
                "ship failed"
            ),
        }
        result
    }

    /// Parse an owned `PyroVec` using the default format.
    fn expose(
        vec: PyroView,
    ) -> Result<
        <<Self::Format as PyroFormat<Self>>::Parser as Parser<PyroView, Self>>::TypedWrapper,
        PyroError,
    > {
        trace!(
            type_name = std::any::type_name::<Self>(),
            len = vec.len(),
            status = ?vec.status(),
            "exposing PyroVec"
        );
        let result = <Self as Bridgeable>::Format::parse(vec);
        match &result {
            Ok(_) => debug!(
                type_name = std::any::type_name::<Self>(),
                "expose completed"
            ),
            Err(e) => warn!(type_name = std::any::type_name::<Self>(), error = %e, "expose failed"),
        }
        result
    }

    /// Parse a borrowed `PyroRef` without taking ownership.
    /// Only available for zero-copy formats (rkyv, zerovec, …).
    fn expose_view<'a>(
        vec: PyroRef<'a>,
    ) -> Result<
        <<Self::Format as PyroFormat<Self>>::RefParser<'a> as Parser<
            PyroRef<'a>,
            Self,
        >>::TypedWrapper,
        PyroError,
    >{
        trace!(
            type_name = std::any::type_name::<Self>(),
            len = vec.len(),
            status = ?vec.status(),
            "exposing PyroRef (zero-copy)"
        );
        let result = <Self::Format as PyroFormat<Self>>::parse_view(vec);
        match &result {
            Ok(_) => debug!(
                type_name = std::any::type_name::<Self>(),
                "expose_view completed"
            ),
            Err(e) => {
                warn!(type_name = std::any::type_name::<Self>(), error = %e, "expose_view failed")
            }
        }
        result
    }
}

// =============================================================================
// BridgeableZeroCopy — expose_view only for zero-copy default formats
// =============================================================================

/// Extension for types whose default format supports zero-copy view parsing.
///
/// This is automatically available when `Self::Format: PyroZeroCopyFormat<Self>`.
/// No extra impl is needed — it's a blanket impl.
pub trait BridgeableZeroCopy: Bridgeable
where
    Self::Format: PyroZeroCopyFormat<Self>,
{
    /// Get a receiver for the default format's parsed type.
    fn receiver() -> <Self::Format as PyroZeroCopyFormat<Self>>::Receiver {
        <Self::Format as PyroZeroCopyFormat<Self>>::receiver()
    }
}

/// Blanket impl: any `Bridgeable` whose default format is zero-copy gets
/// `expose_view` for free.
impl<T> BridgeableZeroCopy for T
where
    T: Bridgeable,
    T::Format: PyroZeroCopyFormat<T>,
{
}

// =============================================================================
// BridgeableResult — Result<T, E> transport (unchanged, uses PyroFormat only)
// =============================================================================

pub trait BridgeableResult<T, E>
where
    T: Bridgeable,
    E: Bridgeable,
{
    fn ship(&self) -> Result<PyroVec, PyroError>;

    fn expose(
        vec: PyroView,
    ) -> Result<
        Result<
            <<T::Format as PyroFormat<T>>::Parser as Parser<PyroView, T>>::TypedWrapper,
            <<E::Format as PyroFormat<E>>::Parser as Parser<PyroView, E>>::TypedWrapper,
        >,
        PyroError,
    >;
}

impl<T, E> BridgeableResult<T, E> for Result<T, E>
where
    T: Bridgeable,
    E: Bridgeable,
{
    fn ship(&self) -> Result<PyroVec, PyroError> {
        match self {
            Ok(value) => {
                trace!(
                    ok_type = std::any::type_name::<T>(),
                    variant = "Ok",
                    version = <T as UserHeaderValues>::VERSION,
                    "shipping Result::Ok"
                );

                let mut vec = T::ship(value)?;
                vec.set_status(<<T as Bridgeable>::Format as PyroFormat<T>>::HeaderValues::OK_CODE);
                Ok(vec)
            }
            Err(error) => {
                trace!(
                    err_type = std::any::type_name::<E>(),
                    variant = "Err",
                    error_version = <E as UserHeaderValues>::VERSION,
                    "shipping Result::Err"
                );
                let mut vec = E::ship(error)?;
                vec.set_status(
                    <<E as Bridgeable>::Format as PyroFormat<E>>::HeaderValues::ERR_CODE,
                );
                Ok(vec)
            }
        }
    }

    fn expose(
        vec: PyroView,
    ) -> Result<
        Result<
            <<T::Format as PyroFormat<T>>::Parser as Parser<PyroView, T>>::TypedWrapper,
            <<E::Format as PyroFormat<E>>::Parser as Parser<PyroView, E>>::TypedWrapper,
        >,
        PyroError,
    > {
        let status = vec.status();

        trace!(
            ok_type = std::any::type_name::<T>(),
            err_type = std::any::type_name::<E>(),
            status = ?status,
            "exposing Result from PyroVec"
        );
        let ok_code = <<T as Bridgeable>::Format as PyroFormat<T>>::HeaderValues::OK_CODE;
        let err_code = <<E as Bridgeable>::Format as PyroFormat<E>>::HeaderValues::ERR_CODE;

        match status {
            Ok(s) if s == ok_code => {
                trace!("Parser::parse_result matched OK_CODE, parsing as success type");
                let parser = <T as Bridgeable>::Format::parser(vec);
                let buf = parser.unchecked_parse()?;
                Ok(Ok(buf))
            }
            Ok(s) if s == err_code => {
                trace!("Parser::parse_result matched ERR_CODE, parsing as error type");
                let parser = <E as Bridgeable>::Format::parser(vec);
                let buf = parser.unchecked_parse()?;
                Ok(Err(buf))
            }
            status => {
                let s = match status {
                    Ok(s) => s as u8,
                    Err(u) => u,
                };
                Err(PyroError::Header(ParseError::UnknownStatus(s)))
            }
        }
    }
}
