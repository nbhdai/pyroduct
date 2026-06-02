use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use tracing::trace;

use crate::{CapturedError, PyroError, PyroResult};

use crate::format::{
    PyroRef, PyroView,
    format::{Parser, Wrapper},
    header::PyroData,
    json::buffers::JsonBuf,
};

// ─── Parser ──────────────────────────────────────────────────────────────────

pub struct JsonParser<BD, T> {
    pub(super) data: BD,
    pub(super) phantom: PhantomData<T>,
}

impl<BD: PyroData, T> Wrapper for JsonParser<BD, T> {
    type Wrapping = BD;
    fn data(&self) -> &Self::Wrapping {
        &self.data
    }
    fn into_data(self) -> Self::Wrapping {
        self.data
    }
}

// ─── PyroVec (owned) ──────────────────────────────────────────────────────

impl<T> Parser<PyroView, T> for JsonParser<PyroView, T>
where
    T: DeserializeOwned,
{
    type HeaderValues = super::JsonHeader;
    /// For JSON the "parsed type" is just `T` — no archived form.
    type ParsedType = T;
    type TypedWrapper = JsonBuf<PyroView, T>;

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

// ─── PyroRef (borrowed) ──────────────────────────────────────────────────

impl<'a, T> Parser<PyroRef<'a>, T> for JsonParser<PyroRef<'a>, T>
where
    T: DeserializeOwned,
{
    type HeaderValues = super::JsonHeader;
    type ParsedType = T;
    type TypedWrapper = JsonBuf<PyroRef<'a>, T>;

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
