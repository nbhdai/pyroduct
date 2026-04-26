//! Bridgeable trait and BridgeableResult extension.

use std::fmt;
use std::panic::Location;

use tracing::trace;

use crate::format::header::{DataStatus, PyroHeader, PyroHeaderMut};
use crate::format::view::PyroView;
use crate::format::{DeepRef, ParseError, PyroVec, ToRow};
use crate::{CapturedError, PyroError, PyroRow};
// =============================================================================
// Bridgeable — default-format convenience (every format)
// =============================================================================

/// A type-safe wrapper around a PyroVec containing an archived rkyv type.
pub struct TypedBuf<T> {
    pub(super) vec: PyroVec,
    pub(super) inner: T,
}

impl<T> TypedBuf<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn view(&self) -> PyroView<'_> {
        self.vec.view()
    }

    pub fn extract_into<S>(self) -> S
    where
        T: Into<S>,
    {
        self.inner.into()
    }

    pub fn extract_owned<S>(self) -> S
    where
        T: ToOwned<Owned = S>,
    {
        self.inner.to_owned()
    }
}

impl<T: fmt::Debug> fmt::Debug for TypedBuf<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// A type-safe wrapper around a PyroVec containing an archived rkyv type.
pub struct TypedView<'a, T: 'a> {
    pub(super) view: PyroView<'a>,
    pub(super) inner: T,
}

impl<'a, T> TypedView<'a, T> {
    pub fn view(&self) -> PyroView<'a> {
        self.view
    }

    pub fn extract<S>(self) -> S
    where
        T: Into<S>,
    {
        self.inner.into()
    }
}

/// A type that has a **default pyro format**.
///
/// This is the main entry point for most users. The `#[bridgeable]` macro
/// generates this impl with `type Format = Rkyv<Self>`.
///
/// For explicit format control, use [`Pyro<T, F>`](crate::pyro::Pyro)
/// instead.
pub trait Bridgeable: Sized {
    type Ref<'a>;
    /// Serialize into a `PyroVec` using the default format.
    fn ship(&self) -> Result<PyroVec, PyroError>;

    /// Parse an owned `PyroVec` using the default format.
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError>;

    /// Parse a borrowed `PyroView` without taking ownership.
    /// Only available for zero-copy formats (rkyv, zerovec, …).
    fn expose_view<'a>(vec: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError>;
}

impl<T: DeepRef + ToRow + 'static> Bridgeable for T
where
    for<'a> T::Ref<'a>: TryFrom<PyroRow<'a>, Error = PyroRow<'a>>,
{
    type Ref<'a> = T::Ref<'a>;

    fn ship(&self) -> Result<PyroVec, PyroError> {
        let row = self.to_row();
        row.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let view = vec.view();
        let row = PyroRow::parse_wire(view)?;
        let value = <T::Ref<'_>>::try_from(row).map_err(|ve| {
            PyroError::deserialization(Box::new(
                CapturedError::new(format!("{ve}")).with_location(Location::caller()),
            ))
        })?;
        // SAFETY:
        // T::Ref is borrowed from PyroVec::data, which this owns.
        let value = unsafe { std::mem::transmute::<T::Ref<'_>, T::Ref<'static>>(value) };
        Ok(TypedBuf { vec, inner: value })
    }

    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let row = PyroRow::parse_wire(view)?;
        let value = <T::Ref<'a>>::try_from(row).map_err(|ve| {
            PyroError::deserialization(Box::new(
                CapturedError::new(format!("{ve}")).with_location(Location::caller()),
            ))
        })?;
        Ok(TypedView { view, inner: value })
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

    fn expose(
        vec: PyroVec,
    ) -> Result<Result<TypedBuf<T::Ref<'static>>, TypedBuf<E::Ref<'static>>>, PyroError>;
    fn expose_view<'a>(
        view: PyroView<'a>,
    ) -> Result<Result<TypedView<'a, T::Ref<'a>>, TypedView<'a, E::Ref<'a>>>, PyroError>;
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

    fn expose(
        vec: PyroVec,
    ) -> Result<Result<TypedBuf<T::Ref<'static>>, TypedBuf<E::Ref<'static>>>, PyroError> {
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

    fn expose_view<'a>(
        view: PyroView<'a>,
    ) -> Result<Result<TypedView<'a, T::Ref<'a>>, TypedView<'a, E::Ref<'a>>>, PyroError> {
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
