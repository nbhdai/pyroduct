use std::marker::PhantomData;

use serde::{Serialize, de::DeserializeOwned};

use crate::format::{
    PyroVec,
    format::{PyroFormat, UserHeaderValues},
    PyroView,
};

use super::{JsonHeader, JsonParser, JsonWriter};

/// JSON pyro format — the serde-based counterpart of [`crate::rkyv_8::Rkyv`].
///
/// Implements [`PyroFormat<T>`] only (not `PyroZeroCopyFormat`) because JSON
/// parsing always produces an owned `T` — there is no archived representation to
/// borrow from the buffer.
pub struct Json<T> {
    phantom: PhantomData<T>,
}

impl<T> PyroFormat<T> for Json<T>
where
    T: UserHeaderValues + Serialize + DeserializeOwned + Clone,
{
    const WIRE_FORMAT: u8 = 1; // Json isn't going to change.

    type HeaderValues = JsonHeader;
    type ParsedType = T;

    type VecParser = JsonParser<PyroVec, T>;
    type ViewParser = JsonParser<PyroView, T>;
    type VecWriter = JsonWriter<PyroVec, T>;

    fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    fn new_writer(data: PyroVec) -> Self::VecWriter {
        JsonWriter {
            data,
            phantom: PhantomData,
        }
    }

    fn vec_parser(data: PyroVec) -> Self::VecParser {
        JsonParser {
            data,
            phantom: PhantomData,
        }
    }
    fn view_parser(data: PyroView) -> Self::ViewParser {
        JsonParser {
            data,
            phantom: PhantomData,
        }
    }
}
