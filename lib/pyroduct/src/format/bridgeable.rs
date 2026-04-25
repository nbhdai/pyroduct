//! Bridgeable trait and BridgeableResult extension.

use std::panic::Location;

use tracing::trace;

use crate::format::header::{DataStatus, PyroHeader, PyroHeaderMut};
use crate::format::value::FromRow;
use crate::format::value::deep_ref::FromDeepRef;
use crate::format::view::PyroView;
use crate::format::{DeepRef, ParseError, PyroVec, ToRow};
use crate::{CapturedError, PyroError, PyroRow};
// =============================================================================
// Bridgeable — default-format convenience (every format)
// =============================================================================

/// A type-safe wrapper around a PyroVec containing an archived rkyv type.
pub struct TypedBuf<T: DeepRef + 'static + ?Sized> {
    pub(super) vec: PyroVec,
    pub(super) inner: T::Ref<'static>,
}

/// A type-safe wrapper around a PyroVec containing an archived rkyv type.
pub struct TypedView<'a, T: DeepRef + 'a + ?Sized> {
    pub(super) view: PyroView<'a>,
    pub(super) inner: T::Ref<'a>,
}

impl<T: DeepRef + 'static> TypedBuf<T> {
    pub fn view(&self) -> PyroView<'_> {
        self.vec.view()
    }

    pub fn extract(&self) -> T
    where
        T: FromDeepRef,
    {
        T::from_ref(&self.inner)
    }
}

impl<'a, T: DeepRef + 'a> TypedView<'a, T> {
    pub fn view(&self) -> PyroView<'a> {
        self.view
    }

    pub fn extract(&self) -> T
    where
        T: FromDeepRef,
    {
        T::from_ref(&self.inner)
    }
}

/// A type that has a **default pyro format**.
///
/// This is the main entry point for most users. The `#[bridgeable]` macro
/// generates this impl with `type Format = Rkyv<Self>`.
///
/// For explicit format control, use [`Pyro<T, F>`](crate::pyro::Pyro)
/// instead.
pub trait Bridgeable: DeepRef + FromDeepRef {
    /// Serialize into a `PyroVec` using the default format.
    fn ship(&self) -> Result<PyroVec, PyroError>;

    /// Parse an owned `PyroVec` using the default format.
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self>, PyroError>;

    /// Parse a borrowed `PyroView` without taking ownership.
    /// Only available for zero-copy formats (rkyv, zerovec, …).
    fn expose_view<'a>(vec: PyroView<'a>) -> Result<TypedView<'a, Self>, PyroError>;
}

impl<T: DeepRef + FromDeepRef + ToRow> Bridgeable for T
where
    for<'a> T::Ref<'a>: FromRow,
{
    fn ship(&self) -> Result<PyroVec, PyroError> {
        let row = self.to_row();
        row.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self>, PyroError> {
        let view = vec.view();
        let row = PyroRow::parse_wire(view)?;
        let value = <T::Ref<'_>>::from_row(row).map_err(|ve| {
            PyroError::deserialization(Box::new(
                CapturedError::new(ve.to_string()).with_location(Location::caller()),
            ))
        })?;
        // SAFETY:
        // T::Ref is borrowed from PyroVec::data, which this owns.
        let value = unsafe { std::mem::transmute::<T::Ref<'_>, T::Ref<'static>>(value) };
        Ok(TypedBuf { vec, inner: value })
    }

    fn expose_view<'a>(vec: PyroView<'a>) -> Result<TypedView<'a, Self>, PyroError> {
        let row = PyroRow::parse_wire(vec)?;
        let value = <T::Ref<'a>>::from_row(row).map_err(|ve| {
            PyroError::deserialization(Box::new(
                CapturedError::new(ve.to_string()).with_location(Location::caller()),
            ))
        })?;
        Ok(TypedView {
            view: vec,
            inner: value,
        })
    }
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

    fn expose(vec: PyroVec) -> Result<Result<TypedBuf<T>, TypedBuf<E>>, PyroError>;
    fn expose_view<'a>(
        vec: PyroView<'a>,
    ) -> Result<Result<TypedView<'a, T>, TypedView<'a, E>>, PyroError>;
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
                    "shipping Result::Ok"
                );

                let mut vec = T::ship(value)?;
                vec.set_status(DataStatus::Valid);
                Ok(vec)
            }
            Err(error) => {
                trace!(
                    err_type = std::any::type_name::<E>(),
                    variant = "Err",
                    "shipping Result::Err"
                );
                let mut vec = E::ship(error)?;
                vec.set_status(DataStatus::Error);
                Ok(vec)
            }
        }
    }

    fn expose(vec: PyroVec) -> Result<Result<TypedBuf<T>, TypedBuf<E>>, PyroError> {
        let status = vec.status();

        trace!(
            ok_type = std::any::type_name::<T>(),
            err_type = std::any::type_name::<E>(),
            status = ?status,
            "exposing Result from PyroVec"
        );

        match status {
            Ok(s) if s == DataStatus::Valid => {
                trace!("Parser::parse_result matched OK_CODE, parsing as success type");
                let buf = T::expose(vec)?;
                Ok(Ok(buf))
            }
            Ok(s) if s == DataStatus::Error => {
                trace!("Parser::parse_result matched ERR_CODE, parsing as error type");
                let buf = E::expose(vec)?;
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

    fn expose_view(
        view: PyroView<'_>,
    ) -> Result<Result<TypedView<'_, T>, TypedView<'_, E>>, PyroError> {
        let status = view.status();

        trace!(
            ok_type = std::any::type_name::<T>(),
            err_type = std::any::type_name::<E>(),
            status = ?status,
            "exposing Result from PyroVec"
        );

        match status {
            Ok(s) if s == DataStatus::Valid => {
                trace!("Parser::parse_result matched OK_CODE, parsing as success type");
                let buf = T::expose_view(view)?;
                Ok(Ok(buf))
            }
            Ok(s) if s == DataStatus::Error => {
                trace!("Parser::parse_result matched ERR_CODE, parsing as error type");
                let buf = E::expose_view(view)?;
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
