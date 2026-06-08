use std::marker::PhantomData;

use serde::{Serialize, de::DeserializeOwned};

use crate::format::{PyroRef, PyroVec, PyroView, format::PyroFormat};

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
    T: Serialize + DeserializeOwned + Clone,
{
    const WIRE_FORMAT: u8 = 1; // Json isn't going to change.

    type HeaderValues = JsonHeader;
    type ParsedType = T;

    type Parser = JsonParser<PyroView, T>;
    type RefParser<'a> = JsonParser<PyroRef<'a>, T>;
    type Writer = JsonWriter<PyroVec, T>;

    fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    fn new_writer(data: PyroVec) -> Self::Writer {
        JsonWriter {
            data,
            phantom: PhantomData,
        }
    }

    fn parser(data: PyroView) -> Self::Parser {
        JsonParser {
            data,
            phantom: PhantomData,
        }
    }
    fn view_parser<'a>(data: PyroRef<'a>) -> Self::RefParser<'a> {
        JsonParser {
            data,
            phantom: PhantomData,
        }
    }
}
