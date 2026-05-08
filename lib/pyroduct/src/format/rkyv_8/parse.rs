use rkyv::rancor::Error as RancorError;
use std::marker::PhantomData;

use rkyv::{
    Archive,
    bytecheck::CheckBytes,
    rancor::Strategy,
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};

use crate::format::{
    PyroRef, PyroVec, PyroView, TypedPyroRef, format::{Parser, UserHeaderValues, Wrapper}, header::PyroData, rkyv_8::TypedBuf
};
use crate::{CapturedError, PyroError, PyroResult};
pub struct RkyvParser<BD, T> {
    pub(super) data: BD,
    pub(super) phantom: PhantomData<T>,
}

impl<BD: PyroData, T> Wrapper for RkyvParser<BD, T> {
    type Wrapping = BD;
    fn data(&self) -> &Self::Wrapping {
        &self.data
    }
    fn into_data(self) -> Self::Wrapping {
        self.data
    }
}

impl<T> Parser<PyroView, T> for RkyvParser<PyroView, T>
where
    T: UserHeaderValues,
    T: Archive,
    T::Archived: 'static,
    T::Archived:
        for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>>,
{
    type HeaderValues = super::RkyvHeader;
    type ParsedType = <T as rkyv::Archive>::Archived;
    type TypedWrapper = TypedBuf<T>;

    fn unchecked_parse(self) -> PyroResult<Self::TypedWrapper> {
        // This is the data. It does not include the header.
        let slice = &*self.data;

        // 2. Validate the data using rkyv
        let archived_ref = rkyv::access::<T::Archived, RancorError>(slice)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        // 3. Extend lifetime to 'static safely.
        //    SAFETY:
        //    - The data is owned by `self.data` (The PyroVec).
        //    - `TypedBuf` takes ownership of `self.data`.
        //    - Therefore the reference `archived_ref` pointing into that data is valid
        //      as long as `TypedBuf` is alive.
        let archived_elided =
            unsafe { std::mem::transmute::<&T::Archived, &'static T::Archived>(archived_ref) };

        Ok(TypedBuf {
            vec: self.data,
            archived: archived_elided,
        })
    }
}

impl<'a, T> Parser<PyroRef<'a>, T> for RkyvParser<PyroRef<'a>, T>
where
    T: UserHeaderValues,
    T: Archive,
    T::Archived: 'static,
    T::Archived:
        for<'b> CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, RancorError>>,
{
    type HeaderValues = super::RkyvHeader;
    type ParsedType = <T as rkyv::Archive>::Archived;
    type TypedWrapper = TypedPyroRef<'a, T>;

    fn unchecked_parse(self) -> PyroResult<Self::TypedWrapper> {
        let slice = &*self.data;

        // 2. Validate the data using rkyv
        let archived_ref = rkyv::access::<T::Archived, RancorError>(slice)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        // 3. Extend lifetime to 'static safely.
        //    SAFETY:
        //    - The data is owned by `self.data` (The PyroVec).
        //    - `TypedBuf` takes ownership of `self.data`.
        //    - Therefore the reference `archived_ref` pointing into that data is valid
        //      as long as `TypedBuf` is alive.
        let archived_elided =
            unsafe { std::mem::transmute::<&T::Archived, &'static T::Archived>(archived_ref) };

        Ok(TypedPyroRef {
            view: self.data,
            archived: archived_elided,
        })
    }
}
