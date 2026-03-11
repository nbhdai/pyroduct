use std::{fmt::Debug, ops::Deref};

use crate::{
    PyroVec,
    format::{HasReceiver, TypedWrapper, Wrapper},
    rkyv_8::RkyvReceiver,
    view::PyroView,
};

/// A type-safe wrapper around a PyroVec containing an archived rkyv type.
pub struct TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    pub(super) vec: PyroVec,
    pub(super) archived: &'static T::Archived,
}

impl<T> Debug for TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.archived.fmt(f)
    }
}

impl<T> Wrapper for TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    type Wrapping = PyroVec;

    fn data(&self) -> &Self::Wrapping {
        &self.vec
    }

    fn into_data(self) -> Self::Wrapping {
        self.vec
    }
}

impl<T> TypedWrapper<<T as rkyv::Archive>::Archived> for TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
}

impl<T> Deref for TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    type Target = <T as rkyv::Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        &self.archived
    }
}

impl<T> HasReceiver<<T as rkyv::Archive>::Archived, T> for TypedBuf<T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    type Receiver = RkyvReceiver<T>;

    fn receiver(&self) -> Self::Receiver {
        RkyvReceiver::new()
    }
}

/// A strongly typed view over a borrowed pyro buffer.
pub struct TypedPyroView<'a, T>
where
    T: rkyv::Archive,
    T::Archived: 'static,
{
    pub(super) view: PyroView<'a>,
    pub(super) archived: &'a T::Archived,
}

impl<'a, T> Debug for TypedPyroView<'a, T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.archived.fmt(f)
    }
}

impl<'a, T> Wrapper for TypedPyroView<'a, T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    type Wrapping = PyroView<'a>;

    fn data(&self) -> &Self::Wrapping {
        &self.view
    }

    fn into_data(self) -> Self::Wrapping {
        self.view
    }
}

impl<'a, T> Deref for TypedPyroView<'a, T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    type Target = <T as rkyv::Archive>::Archived;

    fn deref(&self) -> &Self::Target {
        &self.archived
    }
}

impl<'a, T> TypedWrapper<<T as rkyv::Archive>::Archived> for TypedPyroView<'a, T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
}

impl<'a, T> HasReceiver<<T as rkyv::Archive>::Archived, T> for TypedPyroView<'a, T>
where
    T: rkyv::Archive,
    <T as rkyv::Archive>::Archived: 'static,
{
    type Receiver = RkyvReceiver<T>;

    fn receiver(&self) -> Self::Receiver {
        RkyvReceiver::new()
    }
}
