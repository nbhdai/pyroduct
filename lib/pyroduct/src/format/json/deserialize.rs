use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use tracing::trace;

use crate::format::{Parser, UserHeaderValues, Wrapper};
use crate::json::buffers::JsonBuf;
use crate::view::PyroView;
use crate::{CapturedError, PyroError, PyroResult, PyroVec};

// ─── Parser ──────────────────────────────────────────────────────────────────

pub struct JsonParser<BD, T> {
    pub(super) data: BD,
    pub(super) phantom: PhantomData<T>,
}

impl<BD: crate::header::PyroData, T> Wrapper for JsonParser<BD, T> {
    type Wrapping = BD;
    fn data(&self) -> &Self::Wrapping {
        &self.data
    }
    fn into_data(self) -> Self::Wrapping {
        self.data
    }
}

// ─── PyroVec (owned) ──────────────────────────────────────────────────────

impl<T> Parser<PyroVec, T> for JsonParser<PyroVec, T>
where
    T: UserHeaderValues + DeserializeOwned,
{
    type HeaderValues = super::JsonHeader;
    /// For JSON the "parsed type" is just `T` — no archived form.
    type ParsedType = T;
    type TypedWrapper = JsonBuf<PyroVec, T>;

    fn unchecked_parse(self) -> PyroResult<Self::TypedWrapper> {
        let slice = &*self.data;

        trace!(
            len = slice.len(),
            type_name = std::any::type_name::<T>(),
            "JsonParser::unchecked_parse (vec): deserializing JSON"
        );

        let value: T = serde_json::from_slice(slice)
            .map_err(|e| PyroError::deserialization(CapturedError::new(e)))?;

        Ok(JsonBuf {
            vec: self.data,
            value,
        })
    }
}

// ─── PyroView (borrowed) ──────────────────────────────────────────────────

impl<'a, T> Parser<PyroView<'a>, T> for JsonParser<PyroView<'a>, T>
where
    T: UserHeaderValues + DeserializeOwned,
{
    type HeaderValues = super::JsonHeader;
    type ParsedType = T;
    type TypedWrapper = JsonBuf<PyroView<'a>, T>;

    fn unchecked_parse(self) -> PyroResult<Self::TypedWrapper> {
        let slice = &*self.data;

        trace!(
            len = slice.len(),
            type_name = std::any::type_name::<T>(),
            "JsonParser::unchecked_parse (view): deserializing JSON"
        );

        let value: T = serde_json::from_slice(slice)
            .map_err(|e| PyroError::deserialization(CapturedError::new(e)))?;

        Ok(JsonBuf {
            vec: self.data,
            value,
        })
    }
}
