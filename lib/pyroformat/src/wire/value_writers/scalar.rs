//! [`Scalar`]: one enum over every nesting-0 writer, so the orchestrator can
//! hold them in a single `Vec` next to [`super::nested::Nested`].
//!
//! Variants are the concrete writers from:
//! * `primitive`        — `ScalarWriter<T>`        (`Bool` .. `F64`, `Timestamp`,
//!   and their nullable `…Option` forms)
//! * `fixed`            — `FixedWriter<T>`         (`U8Fixed` .. `F64Fixed`, `BoolFixed`,
//!   and their nullable `…FixedOption` forms)
//!
//! Everything here writes inline into the fixed region at a data offset and
//! never appends payload. The nullable variants also own one bit in the fixed
//! section's validity bitmap, which their `write` sets.

use pyro_schema::{PyroType, StructureWord};

use crate::wire::IoType;
use crate::wire::value_writers::fixed::{
    BoolFixedOptionWriter, F16FixedOptionWriter, F32FixedOptionWriter, F64FixedOptionWriter,
    I8FixedOptionWriter, I16FixedOptionWriter, I32FixedOptionWriter, I64FixedOptionWriter,
    TimestampFixedOptionWriter, U8FixedOptionWriter, U16FixedOptionWriter, U32FixedOptionWriter,
    U64FixedOptionWriter,
};
use crate::wire::value_writers::fixed::{
    BoolFixedWriter, F16FixedWriter, F32FixedWriter, F64FixedWriter, I8FixedWriter, I16FixedWriter,
    I32FixedWriter, I64FixedWriter, TimestampFixedWriter, U8FixedWriter, U16FixedWriter,
    U32FixedWriter, U64FixedWriter,
};
use crate::wire::value_writers::interface::SplitSink;
use crate::wire::value_writers::primitive::{
    BoolWriter, F16Writer, F32Writer, F64Writer, I8Writer, I16Writer, I32Writer, I64Writer,
    OptionBoolWriter, OptionF16Writer, OptionF32Writer, OptionF64Writer, OptionI8Writer,
    OptionI16Writer, OptionI32Writer, OptionI64Writer, OptionTimestampWriter, OptionU8Writer,
    OptionU16Writer, OptionU32Writer, OptionU64Writer, TimestampWriter, U8Writer, U16Writer,
    U32Writer, U64Writer,
};
use crate::{PyroValue, errors::WireError};

#[derive(Debug, Clone)]
pub enum Scalar<'b> {
    // Scalars, inline at the data offset
    Bool(BoolWriter),
    U8(U8Writer),
    I8(I8Writer),
    U16(U16Writer),
    I16(I16Writer),
    U32(U32Writer),
    I32(I32Writer),
    U64(U64Writer),
    I64(I64Writer),
    F16(F16Writer),
    F32(F32Writer),
    F64(F64Writer),
    Timestamp(TimestampWriter),

    // Nullable scalars
    BoolOption(OptionBoolWriter),
    U8Option(OptionU8Writer),
    I8Option(OptionI8Writer),
    U16Option(OptionU16Writer),
    I16Option(OptionI16Writer),
    U32Option(OptionU32Writer),
    I32Option(OptionI32Writer),
    U64Option(OptionU64Writer),
    I64Option(OptionI64Writer),
    F16Option(OptionF16Writer),
    F32Option(OptionF32Writer),
    F64Option(OptionF64Writer),
    TimestampOption(OptionTimestampWriter),

    // Fixed-size arrays
    U8Fixed(U8FixedWriter<'b>),
    I8Fixed(I8FixedWriter<'b>),
    U16Fixed(U16FixedWriter<'b>),
    I16Fixed(I16FixedWriter<'b>),
    U32Fixed(U32FixedWriter<'b>),
    I32Fixed(I32FixedWriter<'b>),
    U64Fixed(U64FixedWriter<'b>),
    I64Fixed(I64FixedWriter<'b>),
    F16Fixed(F16FixedWriter<'b>),
    F32Fixed(F32FixedWriter<'b>),
    F64Fixed(F64FixedWriter<'b>),
    TimestampFixed(TimestampFixedWriter<'b>),
    BoolFixed(BoolFixedWriter<'b>),

    // Nullable fixed-size arrays
    U8FixedOption(U8FixedOptionWriter<'b>),
    I8FixedOption(I8FixedOptionWriter<'b>),
    U16FixedOption(U16FixedOptionWriter<'b>),
    I16FixedOption(I16FixedOptionWriter<'b>),
    U32FixedOption(U32FixedOptionWriter<'b>),
    I32FixedOption(I32FixedOptionWriter<'b>),
    U64FixedOption(U64FixedOptionWriter<'b>),
    I64FixedOption(I64FixedOptionWriter<'b>),
    F16FixedOption(F16FixedOptionWriter<'b>),
    F32FixedOption(F32FixedOptionWriter<'b>),
    F64FixedOption(F64FixedOptionWriter<'b>),
    TimestampFixedOption(TimestampFixedOptionWriter<'b>),
    BoolFixedOption(BoolFixedOptionWriter<'b>),
}

/// `delegate!(self, method, args..)` — one match arm per variant, calling an
/// inherent method every variant shares.
macro_rules! delegate {
    ($self:ident, $method:ident $(, $args:expr)*) => {
        match $self {
            Scalar::Bool(w) => w.$method($($args),*),
            Scalar::U8(w) => w.$method($($args),*),
            Scalar::I8(w) => w.$method($($args),*),
            Scalar::U16(w) => w.$method($($args),*),
            Scalar::I16(w) => w.$method($($args),*),
            Scalar::U32(w) => w.$method($($args),*),
            Scalar::I32(w) => w.$method($($args),*),
            Scalar::U64(w) => w.$method($($args),*),
            Scalar::I64(w) => w.$method($($args),*),
            Scalar::F16(w) => w.$method($($args),*),
            Scalar::F32(w) => w.$method($($args),*),
            Scalar::F64(w) => w.$method($($args),*),
            Scalar::Timestamp(w) => w.$method($($args),*),

            Scalar::BoolOption(w) => w.$method($($args),*),
            Scalar::U8Option(w) => w.$method($($args),*),
            Scalar::I8Option(w) => w.$method($($args),*),
            Scalar::U16Option(w) => w.$method($($args),*),
            Scalar::I16Option(w) => w.$method($($args),*),
            Scalar::U32Option(w) => w.$method($($args),*),
            Scalar::I32Option(w) => w.$method($($args),*),
            Scalar::U64Option(w) => w.$method($($args),*),
            Scalar::I64Option(w) => w.$method($($args),*),
            Scalar::F16Option(w) => w.$method($($args),*),
            Scalar::F32Option(w) => w.$method($($args),*),
            Scalar::F64Option(w) => w.$method($($args),*),
            Scalar::TimestampOption(w) => w.$method($($args),*),

            Scalar::U8Fixed(w) => w.$method($($args),*),
            Scalar::I8Fixed(w) => w.$method($($args),*),
            Scalar::U16Fixed(w) => w.$method($($args),*),
            Scalar::I16Fixed(w) => w.$method($($args),*),
            Scalar::U32Fixed(w) => w.$method($($args),*),
            Scalar::I32Fixed(w) => w.$method($($args),*),
            Scalar::U64Fixed(w) => w.$method($($args),*),
            Scalar::I64Fixed(w) => w.$method($($args),*),
            Scalar::F16Fixed(w) => w.$method($($args),*),
            Scalar::F32Fixed(w) => w.$method($($args),*),
            Scalar::F64Fixed(w) => w.$method($($args),*),
            Scalar::TimestampFixed(w) => w.$method($($args),*),
            Scalar::BoolFixed(w) => w.$method($($args),*),

            Scalar::U8FixedOption(w) => w.$method($($args),*),
            Scalar::I8FixedOption(w) => w.$method($($args),*),
            Scalar::U16FixedOption(w) => w.$method($($args),*),
            Scalar::I16FixedOption(w) => w.$method($($args),*),
            Scalar::U32FixedOption(w) => w.$method($($args),*),
            Scalar::I32FixedOption(w) => w.$method($($args),*),
            Scalar::U64FixedOption(w) => w.$method($($args),*),
            Scalar::I64FixedOption(w) => w.$method($($args),*),
            Scalar::F16FixedOption(w) => w.$method($($args),*),
            Scalar::F32FixedOption(w) => w.$method($($args),*),
            Scalar::F64FixedOption(w) => w.$method($($args),*),
            Scalar::TimestampFixedOption(w) => w.$method($($args),*),
            Scalar::BoolFixedOption(w) => w.$method($($args),*),
        }
    };
}

/// `delegate_trait!(self, Trait::method, args..)` — fully qualified, because
/// `alignment` lives on both `WireSized` and nothing else should guess.
macro_rules! delegate_trait {
    ($self:ident, $tr:ident::$method:ident $(, $args:expr)*) => {
        match $self {
            Scalar::Bool(w) => $tr::$method(w $(, $args)*),
            Scalar::U8(w) => $tr::$method(w $(, $args)*),
            Scalar::I8(w) => $tr::$method(w $(, $args)*),
            Scalar::U16(w) => $tr::$method(w $(, $args)*),
            Scalar::I16(w) => $tr::$method(w $(, $args)*),
            Scalar::U32(w) => $tr::$method(w $(, $args)*),
            Scalar::I32(w) => $tr::$method(w $(, $args)*),
            Scalar::U64(w) => $tr::$method(w $(, $args)*),
            Scalar::I64(w) => $tr::$method(w $(, $args)*),
            Scalar::F16(w) => $tr::$method(w $(, $args)*),
            Scalar::F32(w) => $tr::$method(w $(, $args)*),
            Scalar::F64(w) => $tr::$method(w $(, $args)*),
            Scalar::Timestamp(w) => $tr::$method(w $(, $args)*),

            Scalar::BoolOption(w) => $tr::$method(w $(, $args)*),
            Scalar::U8Option(w) => $tr::$method(w $(, $args)*),
            Scalar::I8Option(w) => $tr::$method(w $(, $args)*),
            Scalar::U16Option(w) => $tr::$method(w $(, $args)*),
            Scalar::I16Option(w) => $tr::$method(w $(, $args)*),
            Scalar::U32Option(w) => $tr::$method(w $(, $args)*),
            Scalar::I32Option(w) => $tr::$method(w $(, $args)*),
            Scalar::U64Option(w) => $tr::$method(w $(, $args)*),
            Scalar::I64Option(w) => $tr::$method(w $(, $args)*),
            Scalar::F16Option(w) => $tr::$method(w $(, $args)*),
            Scalar::F32Option(w) => $tr::$method(w $(, $args)*),
            Scalar::F64Option(w) => $tr::$method(w $(, $args)*),
            Scalar::TimestampOption(w) => $tr::$method(w $(, $args)*),

            Scalar::U8Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::I8Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::U16Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::I16Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::U32Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::I32Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::U64Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::I64Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::F16Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::F32Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::F64Fixed(w) => $tr::$method(w $(, $args)*),
            Scalar::TimestampFixed(w) => $tr::$method(w $(, $args)*),
            Scalar::BoolFixed(w) => $tr::$method(w $(, $args)*),

            Scalar::U8FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::I8FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::U16FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::I16FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::U32FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::I32FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::U64FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::I64FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::F16FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::F32FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::F64FixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::TimestampFixedOption(w) => $tr::$method(w $(, $args)*),
            Scalar::BoolFixedOption(w) => $tr::$method(w $(, $args)*),
        }
    };
}

impl<'b> Scalar<'b> {
    /// Store the one value (or array, or null). Rejects the wrong shape.
    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        delegate!(self, ingest, val)
    }

    /// Write inline at the writer's data offset (a nullable one also sets
    /// its validity bit); nothing is appended, so a scalar has no data
    /// passes and its position among the other writes doesn't matter.
    pub fn write_structure<W: StructureWord>(
        &self,
        buffer: &mut SplitSink<'_, W>,
    ) -> Result<(), WireError> {
        delegate!(self, write, buffer)
    }

    /// Drop the value and any borrow; sizes (`n`) are kept.
    pub fn clear<'c>(self) -> Scalar<'c> {
        match self {
            Scalar::Bool(w) => Scalar::Bool(w.clear()),
            Scalar::U8(w) => Scalar::U8(w.clear()),
            Scalar::I8(w) => Scalar::I8(w.clear()),
            Scalar::U16(w) => Scalar::U16(w.clear()),
            Scalar::I16(w) => Scalar::I16(w.clear()),
            Scalar::U32(w) => Scalar::U32(w.clear()),
            Scalar::I32(w) => Scalar::I32(w.clear()),
            Scalar::U64(w) => Scalar::U64(w.clear()),
            Scalar::I64(w) => Scalar::I64(w.clear()),
            Scalar::F16(w) => Scalar::F16(w.clear()),
            Scalar::F32(w) => Scalar::F32(w.clear()),
            Scalar::F64(w) => Scalar::F64(w.clear()),
            Scalar::Timestamp(w) => Scalar::Timestamp(w.clear()),

            Scalar::BoolOption(w) => Scalar::BoolOption(w.clear()),
            Scalar::U8Option(w) => Scalar::U8Option(w.clear()),
            Scalar::I8Option(w) => Scalar::I8Option(w.clear()),
            Scalar::U16Option(w) => Scalar::U16Option(w.clear()),
            Scalar::I16Option(w) => Scalar::I16Option(w.clear()),
            Scalar::U32Option(w) => Scalar::U32Option(w.clear()),
            Scalar::I32Option(w) => Scalar::I32Option(w.clear()),
            Scalar::U64Option(w) => Scalar::U64Option(w.clear()),
            Scalar::I64Option(w) => Scalar::I64Option(w.clear()),
            Scalar::F16Option(w) => Scalar::F16Option(w.clear()),
            Scalar::F32Option(w) => Scalar::F32Option(w.clear()),
            Scalar::F64Option(w) => Scalar::F64Option(w.clear()),
            Scalar::TimestampOption(w) => Scalar::TimestampOption(w.clear()),

            Scalar::U8Fixed(w) => Scalar::U8Fixed(w.clear()),
            Scalar::I8Fixed(w) => Scalar::I8Fixed(w.clear()),
            Scalar::U16Fixed(w) => Scalar::U16Fixed(w.clear()),
            Scalar::I16Fixed(w) => Scalar::I16Fixed(w.clear()),
            Scalar::U32Fixed(w) => Scalar::U32Fixed(w.clear()),
            Scalar::I32Fixed(w) => Scalar::I32Fixed(w.clear()),
            Scalar::U64Fixed(w) => Scalar::U64Fixed(w.clear()),
            Scalar::I64Fixed(w) => Scalar::I64Fixed(w.clear()),
            Scalar::F16Fixed(w) => Scalar::F16Fixed(w.clear()),
            Scalar::F32Fixed(w) => Scalar::F32Fixed(w.clear()),
            Scalar::F64Fixed(w) => Scalar::F64Fixed(w.clear()),
            Scalar::TimestampFixed(w) => Scalar::TimestampFixed(w.clear()),
            Scalar::BoolFixed(w) => Scalar::BoolFixed(w.clear()),

            Scalar::U8FixedOption(w) => Scalar::U8FixedOption(w.clear()),
            Scalar::I8FixedOption(w) => Scalar::I8FixedOption(w.clear()),
            Scalar::U16FixedOption(w) => Scalar::U16FixedOption(w.clear()),
            Scalar::I16FixedOption(w) => Scalar::I16FixedOption(w.clear()),
            Scalar::U32FixedOption(w) => Scalar::U32FixedOption(w.clear()),
            Scalar::I32FixedOption(w) => Scalar::I32FixedOption(w.clear()),
            Scalar::U64FixedOption(w) => Scalar::U64FixedOption(w.clear()),
            Scalar::I64FixedOption(w) => Scalar::I64FixedOption(w.clear()),
            Scalar::F16FixedOption(w) => Scalar::F16FixedOption(w.clear()),
            Scalar::F32FixedOption(w) => Scalar::F32FixedOption(w.clear()),
            Scalar::F64FixedOption(w) => Scalar::F64FixedOption(w.clear()),
            Scalar::TimestampFixedOption(w) => Scalar::TimestampFixedOption(w.clear()),
            Scalar::BoolFixedOption(w) => Scalar::BoolFixedOption(w.clear()),
        }
    }
}

impl<'b> IoType for Scalar<'b> {
    fn datatype(&self) -> PyroType {
        delegate_trait!(self, IoType::datatype)
    }
}

macro_rules! from_writer {
    ($variant:ident, $ty:ty) => {
        impl<'b> From<$ty> for Scalar<'b> {
            #[inline]
            fn from(w: $ty) -> Self {
                Scalar::$variant(w)
            }
        }
    };
}

from_writer!(Bool, BoolWriter);
from_writer!(U8, U8Writer);
from_writer!(I8, I8Writer);
from_writer!(U16, U16Writer);
from_writer!(I16, I16Writer);
from_writer!(U32, U32Writer);
from_writer!(I32, I32Writer);
from_writer!(U64, U64Writer);
from_writer!(I64, I64Writer);
from_writer!(F16, F16Writer);
from_writer!(F32, F32Writer);
from_writer!(F64, F64Writer);
from_writer!(Timestamp, TimestampWriter);
from_writer!(BoolOption, OptionBoolWriter);
from_writer!(U8Option, OptionU8Writer);
from_writer!(I8Option, OptionI8Writer);
from_writer!(U16Option, OptionU16Writer);
from_writer!(I16Option, OptionI16Writer);
from_writer!(U32Option, OptionU32Writer);
from_writer!(I32Option, OptionI32Writer);
from_writer!(U64Option, OptionU64Writer);
from_writer!(I64Option, OptionI64Writer);
from_writer!(F16Option, OptionF16Writer);
from_writer!(F32Option, OptionF32Writer);
from_writer!(F64Option, OptionF64Writer);
from_writer!(TimestampOption, OptionTimestampWriter);
from_writer!(U8Fixed, U8FixedWriter<'b>);
from_writer!(I8Fixed, I8FixedWriter<'b>);
from_writer!(U16Fixed, U16FixedWriter<'b>);
from_writer!(I16Fixed, I16FixedWriter<'b>);
from_writer!(U32Fixed, U32FixedWriter<'b>);
from_writer!(I32Fixed, I32FixedWriter<'b>);
from_writer!(U64Fixed, U64FixedWriter<'b>);
from_writer!(I64Fixed, I64FixedWriter<'b>);
from_writer!(F16Fixed, F16FixedWriter<'b>);
from_writer!(F32Fixed, F32FixedWriter<'b>);
from_writer!(F64Fixed, F64FixedWriter<'b>);
from_writer!(TimestampFixed, TimestampFixedWriter<'b>);
from_writer!(BoolFixed, BoolFixedWriter<'b>);
from_writer!(U8FixedOption, U8FixedOptionWriter<'b>);
from_writer!(I8FixedOption, I8FixedOptionWriter<'b>);
from_writer!(U16FixedOption, U16FixedOptionWriter<'b>);
from_writer!(I16FixedOption, I16FixedOptionWriter<'b>);
from_writer!(U32FixedOption, U32FixedOptionWriter<'b>);
from_writer!(I32FixedOption, I32FixedOptionWriter<'b>);
from_writer!(U64FixedOption, U64FixedOptionWriter<'b>);
from_writer!(I64FixedOption, I64FixedOptionWriter<'b>);
from_writer!(F16FixedOption, F16FixedOptionWriter<'b>);
from_writer!(F32FixedOption, F32FixedOptionWriter<'b>);
from_writer!(F64FixedOption, F64FixedOptionWriter<'b>);
from_writer!(TimestampFixedOption, TimestampFixedOptionWriter<'b>);
from_writer!(BoolFixedOption, BoolFixedOptionWriter<'b>);
