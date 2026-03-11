use std::marker::PhantomData;

use serde::Serialize;
use tracing::trace;

use crate::format::{
    PyroVec,
    format::{MutWrapper, UserHeaderValues, Wrapper, Writer},
    header::MutPyroData,
};
use crate::{CapturedError, PyroError, PyroResult};

// ─── Writer ──────────────────────────────────────────────────────────────────

pub struct JsonWriter<BD, T> {
    pub(super) data: BD,
    pub(super) phantom: PhantomData<T>,
}

impl<BD: MutPyroData, T> Wrapper for JsonWriter<BD, T> {
    type Wrapping = BD;
    fn data(&self) -> &Self::Wrapping {
        &self.data
    }
    fn into_data(self) -> Self::Wrapping {
        self.data
    }
}

impl<BD: MutPyroData, T> MutWrapper for JsonWriter<BD, T> {
    fn data_mut(&mut self) -> &mut Self::Wrapping {
        &mut self.data
    }
}

impl<T> Writer<PyroVec, T> for JsonWriter<PyroVec, T>
where
    T: UserHeaderValues + Serialize,
{
    type HeaderValues = super::JsonHeader;

    fn write_raw(&mut self, value: &T) -> PyroResult<usize> {
        trace!(
            type_name = std::any::type_name::<T>(),
            "JsonWriter::write_raw: starting JSON serialization"
        );

        let json_bytes = serde_json::to_vec(value)
            .map_err(|e| PyroError::serialization(CapturedError::new(e)))?;

        let len = json_bytes.len();
        self.data.extend_from_slice(&json_bytes);

        trace!(len, "JsonWriter::write_raw: JSON serialization complete");
        Ok(len)
    }
}
