use rkyv::rancor::Error as RancorError;
use std::marker::PhantomData;

use rkyv::{
    Archive,
    bytecheck::CheckBytes,
    rancor::Strategy,
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};

use crate::format::{Parser, UserHeaderValues, Wrapper};
use crate::rkyv_8::TypedBuf;
use crate::view::PyroView;
use crate::{CapturedError, PyroError, PyroResult, PyroVec, TypedPyroView};

pub struct RkyvParser<BD, T> {
    pub(super) data: BD,
    pub(super) phantom: PhantomData<T>,
}

impl<BD: crate::header::PyroData, T> Wrapper for RkyvParser<BD, T> {
    type Wrapping = BD;
    fn data(&self) -> &Self::Wrapping {
        &self.data
    }
    fn into_data(self) -> Self::Wrapping {
        self.data
    }
}

impl<T> Parser<PyroVec, T> for RkyvParser<PyroVec, T>
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

impl<'a, T> Parser<PyroView<'a>, T> for RkyvParser<PyroView<'a>, T>
where
    T: UserHeaderValues,
    T: Archive,
    T::Archived: 'static,
    T::Archived:
        for<'b> CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, RancorError>>,
{
    type HeaderValues = super::RkyvHeader;
    type ParsedType = <T as rkyv::Archive>::Archived;
    type TypedWrapper = TypedPyroView<'a, T>;

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

        Ok(TypedPyroView {
            view: self.data,
            archived: archived_elided,
        })
    }
}
