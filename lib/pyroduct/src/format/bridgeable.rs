//! Bridgeable trait and BridgeableResult extension.

use std::fmt;
use std::ops::Deref;

use crate::PyroError;
use crate::format::header::{DataStatus, PyroHeader, PyroHeaderMut};
use crate::format::view::PyroView;
use crate::format::{ParseError, PyroVec};
// =============================================================================
// Encoder / Decoder Traits
// =============================================================================

pub trait Encoder<T> {
    fn encode(&mut self, val: &T) -> Result<PyroVec, PyroError>;
}

pub trait Decoder<'a, T: 'a> {
    fn decode(&mut self, vec: PyroView<'a>) -> Result<T, PyroError>;
}

pub trait Unpack<Packed>: Sized {
    fn unpack(packed: Packed) -> Result<Self, PyroError>;
}

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

    pub fn into<S>(self) -> S
    where
        S: From<T>,
    {
        self.inner.into()
    }

    pub fn to_owned<S>(self) -> S
    where
        T: ToOwned,
        S: From<T::Owned>,
    {
        self.inner.to_owned().into()
    }
}

impl<T, E> TypedBuf<Result<T, E>> {
    pub fn into_result<S, U>(self) -> Result<S, U>
    where
        S: From<T>,
        U: From<E>,
    {
        match self.inner {
            Ok(ok) => Ok(ok.into()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn to_owned_result<S, U>(self) -> Result<S, U>
    where
        T: ToOwned,
        E: ToOwned,
        S: From<T::Owned>,
        U: From<E::Owned>,
    {
        match self.inner {
            Ok(ok) => Ok(ok.to_owned().into()),
            Err(err) => Err(err.to_owned().into()),
        }
    }
}

impl<T> TypedBuf<Option<T>> {
    pub fn into_option<S>(self) -> Option<S>
    where
        S: From<T>,
    {
        match self.inner {
            Some(ok) => Some(ok.into()),
            None => None,
        }
    }

    pub fn to_owned_option<S>(self) -> Option<S>
    where
        T: ToOwned,
        S: From<T::Owned>,
    {
        match self.inner {
            Some(ok) => Some(ok.to_owned().into()),
            None => None,
        }
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

impl<'a, T> Deref for TypedView<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> TypedView<'a, T> {
    pub fn view(&self) -> PyroView<'a> {
        self.view
    }

    pub fn into<S>(self) -> S
    where
        S: From<T>,
    {
        self.inner.into()
    }

    pub fn to_owned<S>(self) -> S
    where
        T: ToOwned,
        S: From<T::Owned>,
    {
        self.inner.to_owned().into()
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for TypedView<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'a, T, E> TypedView<'a, Result<T, E>> {
    pub fn into_result<S, U>(self) -> Result<S, U>
    where
        S: From<T>,
        U: From<E>,
    {
        match self.inner {
            Ok(ok) => Ok(ok.into()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn to_owned_result<S, U>(self) -> Result<S, U>
    where
        T: ToOwned,
        E: ToOwned,
        S: From<T::Owned>,
        U: From<E::Owned>,
    {
        match self.inner {
            Ok(ok) => Ok(ok.to_owned().into()),
            Err(err) => Err(err.to_owned().into()),
        }
    }
}

impl<'a, T> TypedView<'a, Option<T>> {
    pub fn into_option<S>(self) -> Option<S>
    where
        S: From<T>,
    {
        match self.inner {
            Some(ok) => Some(ok.into()),
            None => None,
        }
    }

    pub fn to_owned_option<S>(self) -> Option<S>
    where
        T: ToOwned,
        S: From<T::Owned>,
    {
        match self.inner {
            Some(ok) => Some(ok.to_owned().into()),
            None => None,
        }
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
    type Encoder: Encoder<Self> + Default;
    type Decoder: for<'a> Decoder<'a, Self::Ref<'a>> + Default;
    type Ref<'a>: 'a;

    /// Serialize into a `PyroVec` using the default format.
    fn ship(&self) -> Result<PyroVec, PyroError> {
        let mut encoder = Self::Encoder::default();
        encoder.encode(self)
    }

    /// Parse an owned `PyroVec` using the default format.
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let mut decoder = Self::Decoder::default();
        let inner = {
            let inner = decoder.decode(vec.view())?;
            unsafe { std::mem::transmute::<Self::Ref<'_>, Self::Ref<'static>>(inner) }
        };
        Ok(TypedBuf { inner, vec })
    }

    /// Parse a borrowed `PyroView` without taking ownership.
    /// Only available for zero-copy formats (rkyv, zerovec, …).
    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let mut decoder = Self::Decoder::default();
        let inner = decoder.decode(view)?;
        Ok(TypedView { inner, view })
    }
}

// =============================================================================
// BridgeableResult — Result<T, E> transport (unchanged, uses PyroFormat only)
// =============================================================================

#[derive(Default)]
pub struct ResultEncoder<T, E> {
    ok_encoder: T,
    err_encoder: E,
}

impl<T, TE: Encoder<T>, E, EE: Encoder<E>> Encoder<Result<T, E>> for ResultEncoder<TE, EE> {
    fn encode(&mut self, val: &Result<T, E>) -> Result<PyroVec, PyroError> {
        match val {
            Ok(value) => {
                let mut vec = self.ok_encoder.encode(value)?;
                vec.set_status(DataStatus::Valid);
                Ok(vec)
            }
            Err(error) => {
                let mut vec = self.err_encoder.encode(error)?;
                vec.set_status(DataStatus::Error);
                Ok(vec)
            }
        }
    }
}

#[derive(Default)]
pub struct ResultDecoder<T, E> {
    ok_decoder: T,
    err_decoder: E,
}

impl<'a, T: 'a, DT: Decoder<'a, T>, E: 'a, DE: Decoder<'a, E>> Decoder<'a, Result<T, E>>
    for ResultDecoder<DT, DE>
{
    fn decode(&mut self, view: PyroView<'a>) -> Result<Result<T, E>, PyroError> {
        let inner = match view.status() {
            Ok(DataStatus::Valid) => Ok(self.ok_decoder.decode(view)?),
            Ok(DataStatus::Error) => Err(self.err_decoder.decode(view)?),
            // Todo: Properly decode the pyro errors.
            Ok(other) => return Err(ParseError::UnknownStatus(other.into()).into()),
            Err(other) => return Err(ParseError::UnknownStatus(other).into()),
        };
        Ok(inner)
    }
}

impl<T: Bridgeable, E: Bridgeable> Bridgeable for Result<T, E> {
    type Encoder = ResultEncoder<T::Encoder, E::Encoder>;

    type Decoder = ResultDecoder<T::Decoder, E::Decoder>;

    type Ref<'a> = Result<T::Ref<'a>, E::Ref<'a>>;
}

#[derive(Default)]
pub struct OptionEncoder<T> {
    inner: T,
}

impl<T, TE: Encoder<T>> Encoder<Option<T>> for OptionEncoder<TE> {
    fn encode(&mut self, val: &Option<T>) -> Result<PyroVec, PyroError> {
        match val {
            Some(value) => {
                let mut vec = self.inner.encode(value)?;
                vec.set_status(DataStatus::Valid);
                Ok(vec)
            }
            None => Ok(PyroVec::ok()),
        }
    }
}

#[derive(Default)]
pub struct OptionDecoder<T> {
    decoder: T,
}

impl<'a, T: 'a, DT: Decoder<'a, T>> Decoder<'a, Option<T>> for OptionDecoder<DT> {
    fn decode(&mut self, view: PyroView<'a>) -> Result<Option<T>, PyroError> {
        let inner = match view.status() {
            Ok(DataStatus::Valid) => Some(self.decoder.decode(view)?),
            Ok(DataStatus::Empty) => None,
            Ok(other) => return Err(ParseError::UnknownStatus(other.into()).into()),
            Err(other) => return Err(ParseError::UnknownStatus(other).into()),
        };
        Ok(inner)
    }
}

impl<T: Bridgeable> Bridgeable for Option<T> {
    type Encoder = OptionEncoder<T::Encoder>;

    type Decoder = OptionDecoder<T::Decoder>;

    type Ref<'a> = Option<T::Ref<'a>>;
}
