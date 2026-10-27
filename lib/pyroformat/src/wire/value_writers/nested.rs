//! [`Nested`]: one enum over every nested leaf writer, so the orchestrator
//! can hold them in a single `Vec` and dispatch [`FeedValue`],
//! [`IngestValue`] and [`WriteLeaf`] without generics.
//!
//! Variants are the concrete writers from:
//! * `nested_slices`        — `NestedValueWriter<T>`   (`U8Slice` .. `F64Slice`,
//!   `TimestampSlice`, `BoolSlice`)
//! * `nested_basic`         — `StrWriter`, `BytesWriter`
//! * `nested_option_slices` — `NestedOptionSliceWriter<T>` (incl. `TimestampOptionSlice`,
//!   `BoolOptionSlice`)
//! * `nested_fixed`         — `NestedFixedWriter<T>` (incl. `BoolFixed`)
//! * `nested_option_fixed`  — `NestedOptionFixedWriter<T>` (incl. `BoolOptionFixed`)
//!
//! `clear` on the enum returns `Nested<'c>` for every variant. The bool
//! writers borrow nothing, so theirs only changes the lifetime parameter.

use pyro_schema::StructureWord;

use crate::errors::WireError;
use crate::schema::PyroType;
use crate::value::PyroValue;
use crate::wire::IoType;
use crate::wire::value_writers::interface::{Padding, SplitSink};
use crate::wire::value_writers::nested_basic::{BytesWriter, StrWriter};
use crate::wire::value_writers::nested_fixed::{
    BoolFixedWriter, F16FixedWriter, F32FixedWriter, F64FixedWriter, I8FixedWriter, I16FixedWriter,
    I32FixedWriter, I64FixedWriter, TimestampFixedWriter, U8FixedWriter, U16FixedWriter,
    U32FixedWriter, U64FixedWriter,
};
use crate::wire::value_writers::nested_option_fixed::{
    BoolOptionFixedWriter, F16OptionFixedWriter, F32OptionFixedWriter, F64OptionFixedWriter,
    I8OptionFixedWriter, I16OptionFixedWriter, I32OptionFixedWriter, I64OptionFixedWriter,
    TimestampOptionFixedWriter, U8OptionFixedWriter, U16OptionFixedWriter, U32OptionFixedWriter,
    U64OptionFixedWriter,
};
use crate::wire::value_writers::nested_option_slices::{
    BoolOptionSliceWriter, F16OptionSliceWriter, F32OptionSliceWriter, F64OptionSliceWriter,
    I8OptionSliceWriter, I16OptionSliceWriter, I32OptionSliceWriter, I64OptionSliceWriter,
    TimestampOptionSliceWriter, U8OptionSliceWriter, U16OptionSliceWriter, U32OptionSliceWriter,
    U64OptionSliceWriter,
};
use crate::wire::value_writers::nested_slices::{
    BoolSliceWriter, F16SliceWriter, F32SliceWriter, F64SliceWriter, FeedValue, I8SliceWriter,
    I16SliceWriter, I32SliceWriter, I64SliceWriter, IngestValue, TimestampSliceWriter,
    U8SliceWriter, U16SliceWriter, U32SliceWriter, U64SliceWriter, WriteLeaf,
};

#[derive(Debug, Clone)]
pub enum Nested<'b, W: StructureWord> {
    // Primitives
    U8Slice(U8SliceWriter<'b, W>),
    I8Slice(I8SliceWriter<'b, W>),
    U16Slice(U16SliceWriter<'b, W>),
    I16Slice(I16SliceWriter<'b, W>),
    U32Slice(U32SliceWriter<'b, W>),
    I32Slice(I32SliceWriter<'b, W>),
    U64Slice(U64SliceWriter<'b, W>),
    I64Slice(I64SliceWriter<'b, W>),
    F16Slice(F16SliceWriter<'b, W>),
    F32Slice(F32SliceWriter<'b, W>),
    F64Slice(F64SliceWriter<'b, W>),
    TimestampSlice(TimestampSliceWriter<'b, W>),
    BoolSlice(BoolSliceWriter<'b, W>),

    // Blobs
    Str(StrWriter<'b, W>),
    Bytes(BytesWriter<'b, W>),

    // Nullable primitives
    U8OptionSlice(U8OptionSliceWriter<'b, W>),
    I8OptionSlice(I8OptionSliceWriter<'b, W>),
    U16OptionSlice(U16OptionSliceWriter<'b, W>),
    I16OptionSlice(I16OptionSliceWriter<'b, W>),
    U32OptionSlice(U32OptionSliceWriter<'b, W>),
    I32OptionSlice(I32OptionSliceWriter<'b, W>),
    U64OptionSlice(U64OptionSliceWriter<'b, W>),
    I64OptionSlice(I64OptionSliceWriter<'b, W>),
    F16OptionSlice(F16OptionSliceWriter<'b, W>),
    F32OptionSlice(F32OptionSliceWriter<'b, W>),
    F64OptionSlice(F64OptionSliceWriter<'b, W>),
    TimestampOptionSlice(TimestampOptionSliceWriter<'b, W>),
    BoolOptionSlice(BoolOptionSliceWriter<'b, W>),

    // Fixed-size arrays
    U8Fixed(U8FixedWriter<'b, W>),
    I8Fixed(I8FixedWriter<'b, W>),
    U16Fixed(U16FixedWriter<'b, W>),
    I16Fixed(I16FixedWriter<'b, W>),
    U32Fixed(U32FixedWriter<'b, W>),
    I32Fixed(I32FixedWriter<'b, W>),
    U64Fixed(U64FixedWriter<'b, W>),
    I64Fixed(I64FixedWriter<'b, W>),
    F16Fixed(F16FixedWriter<'b, W>),
    F32Fixed(F32FixedWriter<'b, W>),
    F64Fixed(F64FixedWriter<'b, W>),
    TimestampFixed(TimestampFixedWriter<'b, W>),
    BoolFixed(BoolFixedWriter<'b, W>),

    // Nullable fixed-size arrays
    U8OptionFixed(U8OptionFixedWriter<'b, W>),
    I8OptionFixed(I8OptionFixedWriter<'b, W>),
    U16OptionFixed(U16OptionFixedWriter<'b, W>),
    I16OptionFixed(I16OptionFixedWriter<'b, W>),
    U32OptionFixed(U32OptionFixedWriter<'b, W>),
    I32OptionFixed(I32OptionFixedWriter<'b, W>),
    U64OptionFixed(U64OptionFixedWriter<'b, W>),
    I64OptionFixed(I64OptionFixedWriter<'b, W>),
    F16OptionFixed(F16OptionFixedWriter<'b, W>),
    F32OptionFixed(F32OptionFixedWriter<'b, W>),
    F64OptionFixed(F64OptionFixedWriter<'b, W>),
    TimestampOptionFixed(TimestampOptionFixedWriter<'b, W>),
    BoolOptionFixed(BoolOptionFixedWriter<'b, W>),
}

/// `delegate!(self, Trait::method, args..)` — one match arm per variant,
/// fully qualified because `alignment` lives on both `WriteLeaf` and `WireSized`.
macro_rules! delegate {
    ($self:ident, $method:ident $(, $args:expr)*) => {
        match $self {
            Nested::U8Slice(w) => w.$method($($args),*),
            Nested::I8Slice(w) => w.$method($($args),*),
            Nested::U16Slice(w) => w.$method($($args),*),
            Nested::I16Slice(w) => w.$method($($args),*),
            Nested::U32Slice(w) => w.$method($($args),*),
            Nested::I32Slice(w) => w.$method($($args),*),
            Nested::U64Slice(w) => w.$method($($args),*),
            Nested::I64Slice(w) => w.$method($($args),*),
            Nested::F16Slice(w) => w.$method($($args),*),
            Nested::F32Slice(w) => w.$method($($args),*),
            Nested::F64Slice(w) => w.$method($($args),*),
            Nested::TimestampSlice(w) => w.$method($($args),*),
            Nested::BoolSlice(w) => w.$method($($args),*),

            Nested::Str(w) => w.$method($($args),*),
            Nested::Bytes(w) => w.$method($($args),*),

            Nested::U8OptionSlice(w) => w.$method($($args),*),
            Nested::I8OptionSlice(w) => w.$method($($args),*),
            Nested::U16OptionSlice(w) => w.$method($($args),*),
            Nested::I16OptionSlice(w) => w.$method($($args),*),
            Nested::U32OptionSlice(w) => w.$method($($args),*),
            Nested::I32OptionSlice(w) => w.$method($($args),*),
            Nested::U64OptionSlice(w) => w.$method($($args),*),
            Nested::I64OptionSlice(w) => w.$method($($args),*),
            Nested::F16OptionSlice(w) => w.$method($($args),*),
            Nested::F32OptionSlice(w) => w.$method($($args),*),
            Nested::F64OptionSlice(w) => w.$method($($args),*),
            Nested::TimestampOptionSlice(w) => w.$method($($args),*),
            Nested::BoolOptionSlice(w) => w.$method($($args),*),

            Nested::U8Fixed(w) => w.$method($($args),*),
            Nested::I8Fixed(w) => w.$method($($args),*),
            Nested::U16Fixed(w) => w.$method($($args),*),
            Nested::I16Fixed(w) => w.$method($($args),*),
            Nested::U32Fixed(w) => w.$method($($args),*),
            Nested::I32Fixed(w) => w.$method($($args),*),
            Nested::U64Fixed(w) => w.$method($($args),*),
            Nested::I64Fixed(w) => w.$method($($args),*),
            Nested::F16Fixed(w) => w.$method($($args),*),
            Nested::F32Fixed(w) => w.$method($($args),*),
            Nested::F64Fixed(w) => w.$method($($args),*),
            Nested::TimestampFixed(w) => w.$method($($args),*),
            Nested::BoolFixed(w) => w.$method($($args),*),

            Nested::U8OptionFixed(w) => w.$method($($args),*),
            Nested::I8OptionFixed(w) => w.$method($($args),*),
            Nested::U16OptionFixed(w) => w.$method($($args),*),
            Nested::I16OptionFixed(w) => w.$method($($args),*),
            Nested::U32OptionFixed(w) => w.$method($($args),*),
            Nested::I32OptionFixed(w) => w.$method($($args),*),
            Nested::U64OptionFixed(w) => w.$method($($args),*),
            Nested::I64OptionFixed(w) => w.$method($($args),*),
            Nested::F16OptionFixed(w) => w.$method($($args),*),
            Nested::F32OptionFixed(w) => w.$method($($args),*),
            Nested::F64OptionFixed(w) => w.$method($($args),*),
            Nested::TimestampOptionFixed(w) => w.$method($($args),*),
            Nested::BoolOptionFixed(w) => w.$method($($args),*),
        }
    };
}

impl<'b, W: StructureWord> Nested<'b, W> {
    pub fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        delegate!(self, feed, val)
    }

    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        delegate!(self, ingest, val)
    }

    /// Empty every accumulator, keep the allocations, drop the borrow.
    #[allow(refining_impl_trait)]
    pub fn clear<'c>(self) -> Nested<'c, W> {
        match self {
            Nested::U8Slice(v) => Nested::U8Slice(v.clear()),
            Nested::I8Slice(v) => Nested::I8Slice(v.clear()),
            Nested::U16Slice(v) => Nested::U16Slice(v.clear()),
            Nested::I16Slice(v) => Nested::I16Slice(v.clear()),
            Nested::U32Slice(v) => Nested::U32Slice(v.clear()),
            Nested::I32Slice(v) => Nested::I32Slice(v.clear()),
            Nested::U64Slice(v) => Nested::U64Slice(v.clear()),
            Nested::I64Slice(v) => Nested::I64Slice(v.clear()),
            Nested::F16Slice(v) => Nested::F16Slice(v.clear()),
            Nested::F32Slice(v) => Nested::F32Slice(v.clear()),
            Nested::F64Slice(v) => Nested::F64Slice(v.clear()),
            Nested::TimestampSlice(v) => Nested::TimestampSlice(v.clear()),
            Nested::BoolSlice(v) => Nested::BoolSlice(v.clear()),

            Nested::Str(v) => Nested::Str(v.clear()),
            Nested::Bytes(v) => Nested::Bytes(v.clear()),

            Nested::U8OptionSlice(v) => Nested::U8OptionSlice(v.clear()),
            Nested::I8OptionSlice(v) => Nested::I8OptionSlice(v.clear()),
            Nested::U16OptionSlice(v) => Nested::U16OptionSlice(v.clear()),
            Nested::I16OptionSlice(v) => Nested::I16OptionSlice(v.clear()),
            Nested::U32OptionSlice(v) => Nested::U32OptionSlice(v.clear()),
            Nested::I32OptionSlice(v) => Nested::I32OptionSlice(v.clear()),
            Nested::U64OptionSlice(v) => Nested::U64OptionSlice(v.clear()),
            Nested::I64OptionSlice(v) => Nested::I64OptionSlice(v.clear()),
            Nested::F16OptionSlice(v) => Nested::F16OptionSlice(v.clear()),
            Nested::F32OptionSlice(v) => Nested::F32OptionSlice(v.clear()),
            Nested::F64OptionSlice(v) => Nested::F64OptionSlice(v.clear()),
            Nested::TimestampOptionSlice(v) => Nested::TimestampOptionSlice(v.clear()),
            Nested::BoolOptionSlice(v) => Nested::BoolOptionSlice(v.clear()),

            Nested::U8Fixed(v) => Nested::U8Fixed(v.clear()),
            Nested::I8Fixed(v) => Nested::I8Fixed(v.clear()),
            Nested::U16Fixed(v) => Nested::U16Fixed(v.clear()),
            Nested::I16Fixed(v) => Nested::I16Fixed(v.clear()),
            Nested::U32Fixed(v) => Nested::U32Fixed(v.clear()),
            Nested::I32Fixed(v) => Nested::I32Fixed(v.clear()),
            Nested::U64Fixed(v) => Nested::U64Fixed(v.clear()),
            Nested::I64Fixed(v) => Nested::I64Fixed(v.clear()),
            Nested::F16Fixed(v) => Nested::F16Fixed(v.clear()),
            Nested::F32Fixed(v) => Nested::F32Fixed(v.clear()),
            Nested::F64Fixed(v) => Nested::F64Fixed(v.clear()),
            Nested::TimestampFixed(v) => Nested::TimestampFixed(v.clear()),
            Nested::BoolFixed(v) => Nested::BoolFixed(v.clear()),

            Nested::U8OptionFixed(v) => Nested::U8OptionFixed(v.clear()),
            Nested::I8OptionFixed(v) => Nested::I8OptionFixed(v.clear()),
            Nested::U16OptionFixed(v) => Nested::U16OptionFixed(v.clear()),
            Nested::I16OptionFixed(v) => Nested::I16OptionFixed(v.clear()),
            Nested::U32OptionFixed(v) => Nested::U32OptionFixed(v.clear()),
            Nested::I32OptionFixed(v) => Nested::I32OptionFixed(v.clear()),
            Nested::U64OptionFixed(v) => Nested::U64OptionFixed(v.clear()),
            Nested::I64OptionFixed(v) => Nested::I64OptionFixed(v.clear()),
            Nested::F16OptionFixed(v) => Nested::F16OptionFixed(v.clear()),
            Nested::F32OptionFixed(v) => Nested::F32OptionFixed(v.clear()),
            Nested::F64OptionFixed(v) => Nested::F64OptionFixed(v.clear()),
            Nested::TimestampOptionFixed(v) => Nested::TimestampOptionFixed(v.clear()),
            Nested::BoolOptionFixed(v) => Nested::BoolOptionFixed(v.clear()),
        }
    }
}

impl<'b, W: StructureWord> Nested<'b, W> {
    pub fn variable_length(&self) -> usize {
        delegate!(self, variable_length)
    }

    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        delegate!(self, write_structure, buffer)
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        delegate!(self, padding_structure, padding)
    }

    /// The `write_N` pass this variant's payload goes out in.
    fn pass(&self) -> usize {
        match self {
            Nested::TimestampSlice(_)
            | Nested::TimestampOptionSlice(_)
            | Nested::TimestampFixed(_)
            | Nested::TimestampOptionFixed(_) => 16,

            Nested::U64Slice(_)
            | Nested::I64Slice(_)
            | Nested::F64Slice(_)
            | Nested::U64OptionSlice(_)
            | Nested::I64OptionSlice(_)
            | Nested::F64OptionSlice(_)
            | Nested::U64Fixed(_)
            | Nested::I64Fixed(_)
            | Nested::F64Fixed(_)
            | Nested::U64OptionFixed(_)
            | Nested::I64OptionFixed(_)
            | Nested::F64OptionFixed(_) => 8,

            Nested::U32Slice(_)
            | Nested::I32Slice(_)
            | Nested::F32Slice(_)
            | Nested::U32OptionSlice(_)
            | Nested::I32OptionSlice(_)
            | Nested::F32OptionSlice(_)
            | Nested::U32Fixed(_)
            | Nested::I32Fixed(_)
            | Nested::F32Fixed(_)
            | Nested::U32OptionFixed(_)
            | Nested::I32OptionFixed(_)
            | Nested::F32OptionFixed(_) => 4,

            Nested::U16Slice(_)
            | Nested::I16Slice(_)
            | Nested::F16Slice(_)
            | Nested::U16OptionSlice(_)
            | Nested::I16OptionSlice(_)
            | Nested::F16OptionSlice(_)
            | Nested::U16Fixed(_)
            | Nested::I16Fixed(_)
            | Nested::F16Fixed(_)
            | Nested::U16OptionFixed(_)
            | Nested::I16OptionFixed(_)
            | Nested::F16OptionFixed(_) => 2,

            Nested::BoolSlice(_)
            | Nested::U8Slice(_)
            | Nested::I8Slice(_)
            | Nested::BoolOptionSlice(_)
            | Nested::U8OptionSlice(_)
            | Nested::I8OptionSlice(_)
            | Nested::U8Fixed(_)
            | Nested::I8Fixed(_)
            | Nested::BoolFixed(_)
            | Nested::U8OptionFixed(_)
            | Nested::I8OptionFixed(_)
            | Nested::BoolOptionFixed(_)
            | Nested::Str(_)
            | Nested::Bytes(_) => 1,
        }
    }

    /// The `write_{pass}` pass's appends, dry.
    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        if self.pass() == pass {
            delegate!(self, padding_data, padding)
        }
    }

    /// The 16-byte data pass: only the variants whose payload is 16-aligned
    /// (`Time`) write here; every pass is the same, at its own alignment.
    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        match self {
            Nested::TimestampSlice(w) => w.write_data(buffer),
            Nested::TimestampOptionSlice(w) => w.write_data(buffer),
            _ => Ok(()),
        }
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        match self {
            Nested::U64Slice(w) => w.write_data(buffer),
            Nested::I64Slice(w) => w.write_data(buffer),
            Nested::F64Slice(w) => w.write_data(buffer),

            Nested::U64OptionSlice(w) => w.write_data(buffer),
            Nested::I64OptionSlice(w) => w.write_data(buffer),
            Nested::F64OptionSlice(w) => w.write_data(buffer),

            Nested::U64Fixed(w) => w.write_data(buffer),
            Nested::I64Fixed(w) => w.write_data(buffer),
            Nested::F64Fixed(w) => w.write_data(buffer),
            Nested::TimestampFixed(w) => w.write_data(buffer),

            Nested::U64OptionFixed(w) => w.write_data(buffer),
            Nested::I64OptionFixed(w) => w.write_data(buffer),
            Nested::F64OptionFixed(w) => w.write_data(buffer),
            Nested::TimestampOptionFixed(w) => w.write_data(buffer),
            _ => Ok(()),
        }
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        match self {
            Nested::U32Slice(w) => w.write_data(buffer),
            Nested::I32Slice(w) => w.write_data(buffer),
            Nested::F32Slice(w) => w.write_data(buffer),

            Nested::U32OptionSlice(w) => w.write_data(buffer),
            Nested::I32OptionSlice(w) => w.write_data(buffer),
            Nested::F32OptionSlice(w) => w.write_data(buffer),

            Nested::U32Fixed(w) => w.write_data(buffer),
            Nested::I32Fixed(w) => w.write_data(buffer),
            Nested::F32Fixed(w) => w.write_data(buffer),

            Nested::U32OptionFixed(w) => w.write_data(buffer),
            Nested::I32OptionFixed(w) => w.write_data(buffer),
            Nested::F32OptionFixed(w) => w.write_data(buffer),
            _ => Ok(()),
        }
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        match self {
            Nested::U16Slice(w) => w.write_data(buffer),
            Nested::I16Slice(w) => w.write_data(buffer),
            Nested::F16Slice(w) => w.write_data(buffer),

            Nested::U16OptionSlice(w) => w.write_data(buffer),
            Nested::I16OptionSlice(w) => w.write_data(buffer),
            Nested::F16OptionSlice(w) => w.write_data(buffer),

            Nested::U16Fixed(w) => w.write_data(buffer),
            Nested::I16Fixed(w) => w.write_data(buffer),
            Nested::F16Fixed(w) => w.write_data(buffer),

            Nested::U16OptionFixed(w) => w.write_data(buffer),
            Nested::I16OptionFixed(w) => w.write_data(buffer),
            Nested::F16OptionFixed(w) => w.write_data(buffer),
            _ => Ok(()),
        }
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        match self {
            Nested::BoolSlice(w) => w.write_data(buffer),
            Nested::U8Slice(w) => w.write_data(buffer),
            Nested::I8Slice(w) => w.write_data(buffer),

            Nested::BoolOptionSlice(w) => w.write_data(buffer),
            Nested::U8OptionSlice(w) => w.write_data(buffer),
            Nested::I8OptionSlice(w) => w.write_data(buffer),

            Nested::U8Fixed(w) => w.write_data(buffer),
            Nested::I8Fixed(w) => w.write_data(buffer),
            Nested::BoolFixed(w) => w.write_data(buffer),

            Nested::U8OptionFixed(w) => w.write_data(buffer),
            Nested::I8OptionFixed(w) => w.write_data(buffer),
            Nested::BoolOptionFixed(w) => w.write_data(buffer),

            Nested::Str(w) => w.write_data(buffer),
            Nested::Bytes(w) => w.write_data(buffer),
            _ => Ok(()),
        }
    }
}

impl<'b, W: StructureWord> IoType for Nested<'b, W> {
    fn datatype(&self) -> PyroType {
        delegate!(self, datatype)
    }
}

macro_rules! from_writer {
    ($variant:ident, $ty:ty) => {
        impl<'b, W: StructureWord> From<$ty> for Nested<'b, W> {
            #[inline]
            fn from(w: $ty) -> Self {
                Nested::$variant(w)
            }
        }
    };
}

from_writer!(U8Slice, U8SliceWriter<'b, W>);
from_writer!(I8Slice, I8SliceWriter<'b, W>);
from_writer!(U16Slice, U16SliceWriter<'b, W>);
from_writer!(I16Slice, I16SliceWriter<'b, W>);
from_writer!(U32Slice, U32SliceWriter<'b, W>);
from_writer!(I32Slice, I32SliceWriter<'b, W>);
from_writer!(U64Slice, U64SliceWriter<'b, W>);
from_writer!(I64Slice, I64SliceWriter<'b, W>);
from_writer!(F16Slice, F16SliceWriter<'b, W>);
from_writer!(F32Slice, F32SliceWriter<'b, W>);
from_writer!(F64Slice, F64SliceWriter<'b, W>);
from_writer!(TimestampSlice, TimestampSliceWriter<'b, W>);
from_writer!(BoolSlice, BoolSliceWriter<'b, W>);
from_writer!(Str, StrWriter<'b, W>);
from_writer!(Bytes, BytesWriter<'b, W>);
from_writer!(U8OptionSlice, U8OptionSliceWriter<'b, W>);
from_writer!(I8OptionSlice, I8OptionSliceWriter<'b, W>);
from_writer!(U16OptionSlice, U16OptionSliceWriter<'b, W>);
from_writer!(I16OptionSlice, I16OptionSliceWriter<'b, W>);
from_writer!(U32OptionSlice, U32OptionSliceWriter<'b, W>);
from_writer!(I32OptionSlice, I32OptionSliceWriter<'b, W>);
from_writer!(U64OptionSlice, U64OptionSliceWriter<'b, W>);
from_writer!(I64OptionSlice, I64OptionSliceWriter<'b, W>);
from_writer!(F16OptionSlice, F16OptionSliceWriter<'b, W>);
from_writer!(F32OptionSlice, F32OptionSliceWriter<'b, W>);
from_writer!(F64OptionSlice, F64OptionSliceWriter<'b, W>);
from_writer!(TimestampOptionSlice, TimestampOptionSliceWriter<'b, W>);
from_writer!(BoolOptionSlice, BoolOptionSliceWriter<'b, W>);
from_writer!(U8Fixed, U8FixedWriter<'b, W>);
from_writer!(I8Fixed, I8FixedWriter<'b, W>);
from_writer!(U16Fixed, U16FixedWriter<'b, W>);
from_writer!(I16Fixed, I16FixedWriter<'b, W>);
from_writer!(U32Fixed, U32FixedWriter<'b, W>);
from_writer!(I32Fixed, I32FixedWriter<'b, W>);
from_writer!(U64Fixed, U64FixedWriter<'b, W>);
from_writer!(I64Fixed, I64FixedWriter<'b, W>);
from_writer!(F16Fixed, F16FixedWriter<'b, W>);
from_writer!(F32Fixed, F32FixedWriter<'b, W>);
from_writer!(F64Fixed, F64FixedWriter<'b, W>);
from_writer!(TimestampFixed, TimestampFixedWriter<'b, W>);
from_writer!(BoolFixed, BoolFixedWriter<'b, W>);
from_writer!(U8OptionFixed, U8OptionFixedWriter<'b, W>);
from_writer!(I8OptionFixed, I8OptionFixedWriter<'b, W>);
from_writer!(U16OptionFixed, U16OptionFixedWriter<'b, W>);
from_writer!(I16OptionFixed, I16OptionFixedWriter<'b, W>);
from_writer!(U32OptionFixed, U32OptionFixedWriter<'b, W>);
from_writer!(I32OptionFixed, I32OptionFixedWriter<'b, W>);
from_writer!(U64OptionFixed, U64OptionFixedWriter<'b, W>);
from_writer!(I64OptionFixed, I64OptionFixedWriter<'b, W>);
from_writer!(F16OptionFixed, F16OptionFixedWriter<'b, W>);
from_writer!(F32OptionFixed, F32OptionFixedWriter<'b, W>);
from_writer!(F64OptionFixed, F64OptionFixedWriter<'b, W>);
from_writer!(TimestampOptionFixed, TimestampOptionFixedWriter<'b, W>);
from_writer!(BoolOptionFixed, BoolOptionFixedWriter<'b, W>);
