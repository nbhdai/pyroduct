//! [`Scalar`]: one enum over every leaf value reader — a primitive, an
//! optional primitive, a fixed-size array (optional or not), a string or a
//! blob, under any number of `Vec`s — so the orchestrator can hold them in a
//! single `Vec` without generics.
//!
//! Every variant ends in its `depth`: how many `Vec`s sit around the value
//! (`0` = the bare value, `1` = `Vec<T>`, …). Fixed-size arrays are
//! `(k, n, depth)`, with `k` the array length; everything else is
//! `(n, depth)`. `n` is the inode.
//!
//! # Depth and topology levels
//!
//! As a root field, a value `depth` lists deep reads `depth - 1` topology
//! levels: the outermost `Vec` is the slot's own `(start, count)`. As a
//! repeated group's column, every row is `depth` lists deep, so the column
//! reads `depth` levels. A string or blob is itself a list of bytes, so it
//! reads one level more in both cases. Views are read at
//! [`MAX_TIERS`] levels and folded at the level count actually in use.

use std::borrow::Cow;

use pyro_schema::{Bow, PrimitiveDataType, StructureWord};

use crate::MAX_TIERS;
use crate::errors::WireError;
use crate::schema::PyroType;
use crate::value::PyroValue;
use crate::wire::IoType;
use crate::wire::value_readers::SplitSource;

use super::{ValueNestedReader, ValueReader};

#[derive(Debug, Clone)]
pub enum Leaf {
    // Blobs: a string is `(n, depth)` like a primitive, but one topology
    // level deeper (see the module doc).
    Str(usize, usize),
    Bytes(usize, usize),

    // Required primitives
    Bool(usize, usize),
    I8(usize, usize),
    I16(usize, usize),
    I32(usize, usize),
    I64(usize, usize),
    U8(usize, usize),
    U16(usize, usize),
    U32(usize, usize),
    U64(usize, usize),
    F16(usize, usize),
    F32(usize, usize),
    F64(usize, usize),
    Timestamp(usize, usize),

    // Optional primitives
    OptionBool(usize, usize),
    OptionI8(usize, usize),
    OptionI16(usize, usize),
    OptionI32(usize, usize),
    OptionI64(usize, usize),
    OptionU8(usize, usize),
    OptionU16(usize, usize),
    OptionU32(usize, usize),
    OptionU64(usize, usize),
    OptionF16(usize, usize),
    OptionF32(usize, usize),
    OptionF64(usize, usize),
    OptionTimestamp(usize, usize),

    // Fixed-size arrays: `(k, n, depth)`
    I8Fixed(usize, usize, usize),
    I16Fixed(usize, usize, usize),
    I32Fixed(usize, usize, usize),
    I64Fixed(usize, usize, usize),
    U8Fixed(usize, usize, usize),
    U16Fixed(usize, usize, usize),
    U32Fixed(usize, usize, usize),
    U64Fixed(usize, usize, usize),
    F16Fixed(usize, usize, usize),
    F32Fixed(usize, usize, usize),
    F64Fixed(usize, usize, usize),
    TimestampFixed(usize, usize, usize),
    BoolFixed(usize, usize, usize),

    // Nullable fixed-size arrays: `(k, n, depth)`
    I8FixedOption(usize, usize, usize),
    I16FixedOption(usize, usize, usize),
    I32FixedOption(usize, usize, usize),
    I64FixedOption(usize, usize, usize),
    U8FixedOption(usize, usize, usize),
    U16FixedOption(usize, usize, usize),
    U32FixedOption(usize, usize, usize),
    U64FixedOption(usize, usize, usize),
    F16FixedOption(usize, usize, usize),
    F32FixedOption(usize, usize, usize),
    F64FixedOption(usize, usize, usize),
    TimestampFixedOption(usize, usize, usize),
    BoolFixedOption(usize, usize, usize),
}

/// A type under `0..LEVELS` `List`s, every level a `'static` that borrows
/// the one inside it, so a [`PyroType`] built from one never allocates.
/// Deep enough for any depth a [`Scalar`] reads at (`MAX_TIERS` topology
/// levels, plus the slot's own list and a blob's byte level).
const LEVELS: usize = 7;
const _: () = assert!(LEVELS >= MAX_TIERS + 2);

type Levels = [&'static PyroType; LEVELS];

/// `[t, List<t>, List<List<t>>, …]` for a `const` `t`, as statics.
macro_rules! static_levels {
    ($leaf:expr) => {{
        static L0: PyroType = $leaf;
        static L1: PyroType = PyroType::List(Bow::Borrowed(&L0));
        static L2: PyroType = PyroType::List(Bow::Borrowed(&L1));
        static L3: PyroType = PyroType::List(Bow::Borrowed(&L2));
        static L4: PyroType = PyroType::List(Bow::Borrowed(&L3));
        static L5: PyroType = PyroType::List(Bow::Borrowed(&L4));
        static L6: PyroType = PyroType::List(Bow::Borrowed(&L5));
        static LEVELS: Levels = [&L0, &L1, &L2, &L3, &L4, &L5, &L6];
        &LEVELS
    }};
}

/// [`static_levels`] for `PyroType::$wrap(pdt)`, one table per primitive.
macro_rules! primitive_levels {
    ($pdt:expr, $wrap:ident) => {
        match $pdt {
            PrimitiveDataType::Bool => static_levels!(PyroType::$wrap(PrimitiveDataType::Bool)),
            PrimitiveDataType::U8 => static_levels!(PyroType::$wrap(PrimitiveDataType::U8)),
            PrimitiveDataType::U16 => static_levels!(PyroType::$wrap(PrimitiveDataType::U16)),
            PrimitiveDataType::U32 => static_levels!(PyroType::$wrap(PrimitiveDataType::U32)),
            PrimitiveDataType::U64 => static_levels!(PyroType::$wrap(PrimitiveDataType::U64)),
            PrimitiveDataType::I8 => static_levels!(PyroType::$wrap(PrimitiveDataType::I8)),
            PrimitiveDataType::I16 => static_levels!(PyroType::$wrap(PrimitiveDataType::I16)),
            PrimitiveDataType::I32 => static_levels!(PyroType::$wrap(PrimitiveDataType::I32)),
            PrimitiveDataType::I64 => static_levels!(PyroType::$wrap(PrimitiveDataType::I64)),
            PrimitiveDataType::F16 => static_levels!(PyroType::$wrap(PrimitiveDataType::F16)),
            PrimitiveDataType::F32 => static_levels!(PyroType::$wrap(PrimitiveDataType::F32)),
            PrimitiveDataType::F64 => static_levels!(PyroType::$wrap(PrimitiveDataType::F64)),
            PrimitiveDataType::Timestamp => {
                static_levels!(PyroType::$wrap(PrimitiveDataType::Timestamp))
            }
        }
    };
}

/// `levels[0]` under `depth` `List`s, borrowed from the table. Only a depth
/// past the table boxes, one level per extra `List`.
fn lists(levels: &'static Levels, depth: usize) -> PyroType {
    match levels.get(depth) {
        Some(t) => (*t).clone(),
        None => boxed_lists(levels[LEVELS - 1].clone(), depth - (LEVELS - 1)),
    }
}

/// `t` under `depth` `List`s, boxing every level: for a type with a runtime
/// part (a fixed-size array's length), which no static table can hold.
fn boxed_lists(mut t: PyroType, depth: usize) -> PyroType {
    for _ in 0..depth {
        t = PyroType::List(Bow::new(t));
    }
    t
}

/// A required primitive `depth` lists deep: the scalar at depth 0, and the
/// innermost list a `PrimitiveList` above that.
fn primitive(pdt: PrimitiveDataType, depth: usize) -> PyroType {
    match depth {
        0 => PyroType::PrimitiveScalar(pdt),
        _ => lists(primitive_levels!(pdt, PrimitiveList), depth - 1),
    }
}

/// A nullable primitive `depth` lists deep.
fn option_primitive(pdt: PrimitiveDataType, depth: usize) -> PyroType {
    lists(primitive_levels!(pdt, OptionPrimitiveScalar), depth)
}

impl IoType for Leaf {
    fn datatype(&self) -> PyroType {
        match self {
            Leaf::Str(_, depth) => lists(static_levels!(PyroType::Str), *depth),
            Leaf::Bytes(_, depth) => lists(static_levels!(PyroType::Bytes), *depth),
            Leaf::Bool(_, depth) => primitive(PrimitiveDataType::Bool, *depth),
            Leaf::I8(_, depth) => primitive(PrimitiveDataType::I8, *depth),
            Leaf::I16(_, depth) => primitive(PrimitiveDataType::I16, *depth),
            Leaf::I32(_, depth) => primitive(PrimitiveDataType::I32, *depth),
            Leaf::I64(_, depth) => primitive(PrimitiveDataType::I64, *depth),
            Leaf::U8(_, depth) => primitive(PrimitiveDataType::U8, *depth),
            Leaf::U16(_, depth) => primitive(PrimitiveDataType::U16, *depth),
            Leaf::U32(_, depth) => primitive(PrimitiveDataType::U32, *depth),
            Leaf::U64(_, depth) => primitive(PrimitiveDataType::U64, *depth),
            Leaf::F16(_, depth) => primitive(PrimitiveDataType::F16, *depth),
            Leaf::F32(_, depth) => primitive(PrimitiveDataType::F32, *depth),
            Leaf::F64(_, depth) => primitive(PrimitiveDataType::F64, *depth),
            Leaf::Timestamp(_, depth) => primitive(PrimitiveDataType::Timestamp, *depth),
            Leaf::OptionBool(_, depth) => option_primitive(PrimitiveDataType::Bool, *depth),
            Leaf::OptionI8(_, depth) => option_primitive(PrimitiveDataType::I8, *depth),
            Leaf::OptionI16(_, depth) => option_primitive(PrimitiveDataType::I16, *depth),
            Leaf::OptionI32(_, depth) => option_primitive(PrimitiveDataType::I32, *depth),
            Leaf::OptionI64(_, depth) => option_primitive(PrimitiveDataType::I64, *depth),
            Leaf::OptionU8(_, depth) => option_primitive(PrimitiveDataType::U8, *depth),
            Leaf::OptionU16(_, depth) => option_primitive(PrimitiveDataType::U16, *depth),
            Leaf::OptionU32(_, depth) => option_primitive(PrimitiveDataType::U32, *depth),
            Leaf::OptionU64(_, depth) => option_primitive(PrimitiveDataType::U64, *depth),
            Leaf::OptionF16(_, depth) => option_primitive(PrimitiveDataType::F16, *depth),
            Leaf::OptionF32(_, depth) => option_primitive(PrimitiveDataType::F32, *depth),
            Leaf::OptionF64(_, depth) => option_primitive(PrimitiveDataType::F64, *depth),
            Leaf::OptionTimestamp(_, depth) => {
                option_primitive(PrimitiveDataType::Timestamp, *depth)
            }
            Leaf::I8Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::I8, *k),
                *depth,
            ),
            Leaf::I16Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::I16, *k),
                *depth,
            ),
            Leaf::I32Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::I32, *k),
                *depth,
            ),
            Leaf::I64Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::I64, *k),
                *depth,
            ),
            Leaf::U8Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::U8, *k),
                *depth,
            ),
            Leaf::U16Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::U16, *k),
                *depth,
            ),
            Leaf::U32Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::U32, *k),
                *depth,
            ),
            Leaf::U64Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::U64, *k),
                *depth,
            ),
            Leaf::F16Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::F16, *k),
                *depth,
            ),
            Leaf::F32Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::F32, *k),
                *depth,
            ),
            Leaf::F64Fixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::F64, *k),
                *depth,
            ),
            Leaf::TimestampFixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::Timestamp, *k),
                *depth,
            ),
            Leaf::BoolFixed(k, _, depth) => boxed_lists(
                PyroType::PrimitiveFixedList(PrimitiveDataType::Bool, *k),
                *depth,
            ),
            Leaf::I8FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::I8, *k),
                *depth,
            ),
            Leaf::I16FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::I16, *k),
                *depth,
            ),
            Leaf::I32FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::I32, *k),
                *depth,
            ),
            Leaf::I64FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::I64, *k),
                *depth,
            ),
            Leaf::U8FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::U8, *k),
                *depth,
            ),
            Leaf::U16FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::U16, *k),
                *depth,
            ),
            Leaf::U32FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::U32, *k),
                *depth,
            ),
            Leaf::U64FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::U64, *k),
                *depth,
            ),
            Leaf::F16FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::F16, *k),
                *depth,
            ),
            Leaf::F32FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::F32, *k),
                *depth,
            ),
            Leaf::F64FixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::F64, *k),
                *depth,
            ),
            Leaf::TimestampFixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::Timestamp, *k),
                *depth,
            ),
            Leaf::BoolFixedOption(k, _, depth) => boxed_lists(
                PyroType::OptionPrimitiveFixedList(PrimitiveDataType::Bool, *k),
                *depth,
            ),
        }
    }
}

/// `levels` topology levels must fit the [`MAX_TIERS`] a view is read at.
macro_rules! check_levels {
    ($levels:expr) => {
        if $levels > MAX_TIERS {
            return Err(WireError::InvalidTopologyDepth { max: MAX_TIERS });
        }
    };
}

/// A required primitive column's rows at `levels` topology levels: each
/// element at `0`, else the levels folded over its `&[T]` lists.
macro_rules! primitive_rows {
    ($buffer:ident, $n:expr, $levels:expr, $slice:ident, $nest:ident) => {
        match $levels {
            0 => $buffer
                .$slice($n)?
                .iter()
                .map(|v| PyroValue::from(*v))
                .collect(),
            levels => {
                check_levels!(levels);
                $buffer.$nest($n, levels)?.into_value(levels)
            }
        }
    };
}

/// [`primitive_rows`] for a nullable-element column: each sample a scalar
/// or `PyroValue::Null`.
macro_rules! option_rows {
    ($buffer:ident, $n:expr, $levels:expr, $slice:ident, $nest:ident) => {
        match $levels {
            0 => match $buffer.$slice($n)?.into_value() {
                PyroValue::List(samples) => samples,
                _ => unreachable!("an option slice is a `List`"),
            },
            levels => {
                check_levels!(levels);
                $buffer.$nest($n, levels)?.into_value(levels)
            }
        }
    };
}

/// A `[T; k]` column's rows at `levels` topology levels over its arrays.
macro_rules! fixed_rows {
    ($buffer:ident, $k:expr, $n:expr, $levels:expr, $fixed_slice:ident) => {{
        let arrays = $buffer.$fixed_slice($n, $k)?;
        fold_leaves($buffer, $n, $levels, arrays.iter().map(PyroValue::from))?
    }};
}

/// [`fixed_rows`] for a nullable `[T; k]` column: each sample an array or
/// `PyroValue::Null`.
macro_rules! fixed_option_rows {
    ($buffer:ident, $k:expr, $n:expr, $levels:expr, $fixed_option_slice:ident) => {{
        let arrays = $buffer.$fixed_option_slice($n, $k)?;
        let samples = arrays
            .iter()
            .map(|v| v.map_or(PyroValue::Null, PyroValue::from));
        fold_leaves($buffer, $n, $levels, samples)?
    }};
}

/// One value per leaf (an array, or a null) at `levels` topology levels:
/// the leaves themselves at `0`, else inode `n`'s levels folded over them —
/// the last level counts leaves, so nothing is split off first.
fn fold_leaves<'a, W: StructureWord>(
    buffer: &SplitSource<'a, W>,
    n: usize,
    levels: usize,
    mut leaves: impl Iterator<Item = PyroValue<'a>>,
) -> Result<Vec<PyroValue<'a>>, WireError> {
    if levels == 0 {
        return Ok(leaves.collect());
    }
    check_levels!(levels);
    buffer.read_topology(n, levels)?.eat_values(&mut leaves)
}

impl ValueReader for Leaf {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_value<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroValue<'a>, WireError> {
        Ok(match self {
            // The bare value.
            Leaf::Str(n, 0) => PyroValue::Str(Cow::Borrowed(buffer.read_str(*n)?)),
            Leaf::Bytes(n, 0) => PyroValue::Bytes(Cow::Borrowed(buffer.read_u8_slice(*n)?)),
            Leaf::Bool(n, 0) => PyroValue::Bool(buffer.read_bool(*n)),
            Leaf::I8(n, 0) => PyroValue::I8(buffer.read_i8(*n)),
            Leaf::I16(n, 0) => PyroValue::I16(buffer.read_i16(*n)),
            Leaf::I32(n, 0) => PyroValue::I32(buffer.read_i32(*n)),
            Leaf::I64(n, 0) => PyroValue::I64(buffer.read_i64(*n)),
            Leaf::U8(n, 0) => PyroValue::U8(buffer.read_u8(*n)),
            Leaf::U16(n, 0) => PyroValue::U16(buffer.read_u16(*n)),
            Leaf::U32(n, 0) => PyroValue::U32(buffer.read_u32(*n)),
            Leaf::U64(n, 0) => PyroValue::U64(buffer.read_u64(*n)),
            Leaf::F16(n, 0) => PyroValue::F16(buffer.read_f16(*n)),
            Leaf::F32(n, 0) => PyroValue::F32(buffer.read_f32(*n)),
            Leaf::F64(n, 0) => PyroValue::F64(buffer.read_f64(*n)),
            Leaf::Timestamp(n, 0) => PyroValue::Timestamp(buffer.read_time(*n)),
            Leaf::OptionBool(n, 0) => match buffer.read_nullable(*n) {
                true => PyroValue::Bool(buffer.read_bool(*n)),
                false => PyroValue::Null,
            },
            Leaf::OptionI8(n, 0) => buffer.read_option_i8(*n).into(),
            Leaf::OptionI16(n, 0) => buffer.read_option_i16(*n).into(),
            Leaf::OptionI32(n, 0) => buffer.read_option_i32(*n).into(),
            Leaf::OptionI64(n, 0) => buffer.read_option_i64(*n).into(),
            Leaf::OptionU8(n, 0) => buffer.read_option_u8(*n).into(),
            Leaf::OptionU16(n, 0) => buffer.read_option_u16(*n).into(),
            Leaf::OptionU32(n, 0) => buffer.read_option_u32(*n).into(),
            Leaf::OptionU64(n, 0) => buffer.read_option_u64(*n).into(),
            Leaf::OptionF16(n, 0) => buffer.read_option_f16(*n).into(),
            Leaf::OptionF32(n, 0) => buffer.read_option_f32(*n).into(),
            Leaf::OptionF64(n, 0) => buffer.read_option_f64(*n).into(),
            Leaf::OptionTimestamp(n, 0) => buffer.read_option_time(*n).into(),
            Leaf::I8Fixed(k, n, 0) => buffer.read_fixed_i8(*n, *k)?.into(),
            Leaf::I16Fixed(k, n, 0) => buffer.read_fixed_i16(*n, *k)?.into(),
            Leaf::I32Fixed(k, n, 0) => buffer.read_fixed_i32(*n, *k)?.into(),
            Leaf::I64Fixed(k, n, 0) => buffer.read_fixed_i64(*n, *k)?.into(),
            Leaf::U8Fixed(k, n, 0) => buffer.read_fixed_u8(*n, *k)?.into(),
            Leaf::U16Fixed(k, n, 0) => buffer.read_fixed_u16(*n, *k)?.into(),
            Leaf::U32Fixed(k, n, 0) => buffer.read_fixed_u32(*n, *k)?.into(),
            Leaf::U64Fixed(k, n, 0) => buffer.read_fixed_u64(*n, *k)?.into(),
            Leaf::F16Fixed(k, n, 0) => buffer.read_fixed_f16(*n, *k)?.into(),
            Leaf::F32Fixed(k, n, 0) => buffer.read_fixed_f32(*n, *k)?.into(),
            Leaf::F64Fixed(k, n, 0) => buffer.read_fixed_f64(*n, *k)?.into(),
            Leaf::TimestampFixed(k, n, 0) => buffer.read_fixed_time(*n, *k)?.into(),
            Leaf::BoolFixed(k, n, 0) => buffer.read_fixed_bool(*n, *k)?.into(),
            Leaf::I8FixedOption(k, n, 0) => buffer
                .read_option_fixed_i8(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::I16FixedOption(k, n, 0) => buffer
                .read_option_fixed_i16(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::I32FixedOption(k, n, 0) => buffer
                .read_option_fixed_i32(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::I64FixedOption(k, n, 0) => buffer
                .read_option_fixed_i64(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::U8FixedOption(k, n, 0) => buffer
                .read_option_fixed_u8(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::U16FixedOption(k, n, 0) => buffer
                .read_option_fixed_u16(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::U32FixedOption(k, n, 0) => buffer
                .read_option_fixed_u32(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::U64FixedOption(k, n, 0) => buffer
                .read_option_fixed_u64(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::F16FixedOption(k, n, 0) => buffer
                .read_option_fixed_f16(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::F32FixedOption(k, n, 0) => buffer
                .read_option_fixed_f32(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::F64FixedOption(k, n, 0) => buffer
                .read_option_fixed_f64(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::TimestampFixedOption(k, n, 0) => buffer
                .read_option_fixed_time(*n, *k)?
                .map_or(PyroValue::Null, PyroValue::from),
            Leaf::BoolFixedOption(k, n, 0) => match buffer.read_nullable(*n) {
                true => buffer.read_fixed_bool(*n, *k)?.into(),
                false => PyroValue::Null,
            },

            // A required primitive list: one borrowed `PrimitiveList`.
            Leaf::Bool(n, 1) => buffer.read_bool_slice(*n)?.into(),
            Leaf::I8(n, 1) => buffer.read_i8_slice(*n)?.into(),
            Leaf::I16(n, 1) => buffer.read_i16_slice(*n)?.into(),
            Leaf::I32(n, 1) => buffer.read_i32_slice(*n)?.into(),
            Leaf::I64(n, 1) => buffer.read_i64_slice(*n)?.into(),
            Leaf::U8(n, 1) => buffer.read_u8_slice(*n)?.into(),
            Leaf::U16(n, 1) => buffer.read_u16_slice(*n)?.into(),
            Leaf::U32(n, 1) => buffer.read_u32_slice(*n)?.into(),
            Leaf::U64(n, 1) => buffer.read_u64_slice(*n)?.into(),
            Leaf::F16(n, 1) => buffer.read_f16_slice(*n)?.into(),
            Leaf::F32(n, 1) => buffer.read_f32_slice(*n)?.into(),
            Leaf::F64(n, 1) => buffer.read_f64_slice(*n)?.into(),
            Leaf::Timestamp(n, 1) => buffer.read_time_slice(*n)?.into(),

            // A `List` of rows: `depth - 1` topology levels (the slot's own
            // `(start, count)` is the outermost list), or `depth` for a blob.
            Leaf::Str(n, depth) => PyroValue::List({
                check_levels!(*depth);
                buffer.read_str_nest(*n, *depth)?.into_value(*depth)
            }),
            Leaf::Bytes(n, depth) => PyroValue::List({
                check_levels!(*depth);
                buffer
                    .read_u8_nest(*n, *depth)?
                    .into_value_with(*depth, |b| PyroValue::Bytes(Cow::Borrowed(b)))
            }),
            Leaf::Bool(n, depth) => PyroValue::List(match *depth - 1 {
                0 => buffer
                    .read_bool_slice(*n)?
                    .iter()
                    .by_vals()
                    .map(PyroValue::Bool)
                    .collect(),
                levels => {
                    check_levels!(levels);
                    buffer.read_bool_nest(*n, levels)?.into_value(levels)
                }
            }),
            Leaf::I8(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i8_slice,
                read_i8_nest
            )),
            Leaf::I16(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i16_slice,
                read_i16_nest
            )),
            Leaf::I32(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i32_slice,
                read_i32_nest
            )),
            Leaf::I64(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i64_slice,
                read_i64_nest
            )),
            Leaf::U8(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u8_slice,
                read_u8_nest
            )),
            Leaf::U16(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u16_slice,
                read_u16_nest
            )),
            Leaf::U32(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u32_slice,
                read_u32_nest
            )),
            Leaf::U64(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u64_slice,
                read_u64_nest
            )),
            Leaf::F16(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_f16_slice,
                read_f16_nest
            )),
            Leaf::F32(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_f32_slice,
                read_f32_nest
            )),
            Leaf::F64(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_f64_slice,
                read_f64_nest
            )),
            Leaf::Timestamp(n, depth) => PyroValue::List(primitive_rows!(
                buffer,
                *n,
                *depth - 1,
                read_time_slice,
                read_time_nest
            )),
            Leaf::OptionBool(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_bool_option_slice,
                read_bool_option_nest
            )),
            Leaf::OptionI8(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i8_option_slice,
                read_i8_option_nest
            )),
            Leaf::OptionI16(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i16_option_slice,
                read_i16_option_nest
            )),
            Leaf::OptionI32(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i32_option_slice,
                read_i32_option_nest
            )),
            Leaf::OptionI64(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_i64_option_slice,
                read_i64_option_nest
            )),
            Leaf::OptionU8(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u8_option_slice,
                read_u8_option_nest
            )),
            Leaf::OptionU16(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u16_option_slice,
                read_u16_option_nest
            )),
            Leaf::OptionU32(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u32_option_slice,
                read_u32_option_nest
            )),
            Leaf::OptionU64(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_u64_option_slice,
                read_u64_option_nest
            )),
            Leaf::OptionF16(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_f16_option_slice,
                read_f16_option_nest
            )),
            Leaf::OptionF32(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_f32_option_slice,
                read_f32_option_nest
            )),
            Leaf::OptionF64(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_f64_option_slice,
                read_f64_option_nest
            )),
            Leaf::OptionTimestamp(n, depth) => PyroValue::List(option_rows!(
                buffer,
                *n,
                *depth - 1,
                read_time_option_slice,
                read_time_option_nest
            )),
            Leaf::I8Fixed(k, n, depth) => {
                PyroValue::List(fixed_rows!(buffer, *k, *n, *depth - 1, read_fixed_i8_slice))
            }
            Leaf::I16Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i16_slice
            )),
            Leaf::I32Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i32_slice
            )),
            Leaf::I64Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i64_slice
            )),
            Leaf::U8Fixed(k, n, depth) => {
                PyroValue::List(fixed_rows!(buffer, *k, *n, *depth - 1, read_fixed_u8_slice))
            }
            Leaf::U16Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u16_slice
            )),
            Leaf::U32Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u32_slice
            )),
            Leaf::U64Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u64_slice
            )),
            Leaf::F16Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_f16_slice
            )),
            Leaf::F32Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_f32_slice
            )),
            Leaf::F64Fixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_f64_slice
            )),
            Leaf::TimestampFixed(k, n, depth) => PyroValue::List(fixed_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_time_slice
            )),
            Leaf::BoolFixed(k, n, depth) => PyroValue::List({
                let bits = buffer.read_fixed_bool_slice(*n, *k)?;
                let arrays = bits.chunks_exact((*k).max(1)).map(PyroValue::from);
                fold_leaves(buffer, *n, *depth - 1, arrays)?
            }),
            Leaf::I8FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i8_option_slice
            )),
            Leaf::I16FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i16_option_slice
            )),
            Leaf::I32FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i32_option_slice
            )),
            Leaf::I64FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_i64_option_slice
            )),
            Leaf::U8FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u8_option_slice
            )),
            Leaf::U16FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u16_option_slice
            )),
            Leaf::U32FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u32_option_slice
            )),
            Leaf::U64FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_u64_option_slice
            )),
            Leaf::F16FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_f16_option_slice
            )),
            Leaf::F32FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_f32_option_slice
            )),
            Leaf::F64FixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_f64_option_slice
            )),
            Leaf::TimestampFixedOption(k, n, depth) => PyroValue::List(fixed_option_rows!(
                buffer,
                *k,
                *n,
                *depth - 1,
                read_fixed_time_option_slice
            )),
            Leaf::BoolFixedOption(k, n, depth) => PyroValue::List({
                let (bits, option) = buffer.read_fixed_bool_option_slice(*n, *k)?;
                let mut present = 0;
                let samples = option.iter().by_vals().map(|some| {
                    if !some {
                        return PyroValue::Null;
                    }
                    let array = &bits[present * k..(present + 1) * k];
                    present += 1;
                    PyroValue::from(array)
                });
                fold_leaves(buffer, *n, *depth - 1, samples)?
            }),
        })
    }
}

impl ValueNestedReader for Leaf {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_values<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<Vec<PyroValue<'a>>, WireError> {
        Ok(match self {
            // Every row is `depth` lists deep: `depth` topology levels, or
            // one more for a blob.
            Leaf::Str(n, depth) => {
                check_levels!(*depth + 1);
                buffer.read_str_nest(*n, *depth + 1)?.into_value(*depth + 1)
            }
            Leaf::Bytes(n, depth) => {
                check_levels!(*depth + 1);
                buffer
                    .read_u8_nest(*n, *depth + 1)?
                    .into_value_with(*depth + 1, |b| PyroValue::Bytes(Cow::Borrowed(b)))
            }
            Leaf::Bool(n, depth) => match *depth {
                0 => buffer
                    .read_bool_slice(*n)?
                    .iter()
                    .by_vals()
                    .map(PyroValue::Bool)
                    .collect(),
                levels => {
                    check_levels!(levels);
                    buffer.read_bool_nest(*n, levels)?.into_value(levels)
                }
            },
            Leaf::I8(n, depth) => primitive_rows!(buffer, *n, *depth, read_i8_slice, read_i8_nest),
            Leaf::I16(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_i16_slice, read_i16_nest)
            }
            Leaf::I32(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_i32_slice, read_i32_nest)
            }
            Leaf::I64(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_i64_slice, read_i64_nest)
            }
            Leaf::U8(n, depth) => primitive_rows!(buffer, *n, *depth, read_u8_slice, read_u8_nest),
            Leaf::U16(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_u16_slice, read_u16_nest)
            }
            Leaf::U32(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_u32_slice, read_u32_nest)
            }
            Leaf::U64(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_u64_slice, read_u64_nest)
            }
            Leaf::F16(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_f16_slice, read_f16_nest)
            }
            Leaf::F32(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_f32_slice, read_f32_nest)
            }
            Leaf::F64(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_f64_slice, read_f64_nest)
            }
            Leaf::Timestamp(n, depth) => {
                primitive_rows!(buffer, *n, *depth, read_time_slice, read_time_nest)
            }
            Leaf::OptionBool(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_bool_option_slice,
                read_bool_option_nest
            ),
            Leaf::OptionI8(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_i8_option_slice,
                read_i8_option_nest
            ),
            Leaf::OptionI16(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_i16_option_slice,
                read_i16_option_nest
            ),
            Leaf::OptionI32(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_i32_option_slice,
                read_i32_option_nest
            ),
            Leaf::OptionI64(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_i64_option_slice,
                read_i64_option_nest
            ),
            Leaf::OptionU8(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_u8_option_slice,
                read_u8_option_nest
            ),
            Leaf::OptionU16(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_u16_option_slice,
                read_u16_option_nest
            ),
            Leaf::OptionU32(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_u32_option_slice,
                read_u32_option_nest
            ),
            Leaf::OptionU64(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_u64_option_slice,
                read_u64_option_nest
            ),
            Leaf::OptionF16(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_f16_option_slice,
                read_f16_option_nest
            ),
            Leaf::OptionF32(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_f32_option_slice,
                read_f32_option_nest
            ),
            Leaf::OptionF64(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_f64_option_slice,
                read_f64_option_nest
            ),
            Leaf::OptionTimestamp(n, depth) => option_rows!(
                buffer,
                *n,
                *depth,
                read_time_option_slice,
                read_time_option_nest
            ),
            Leaf::I8Fixed(k, n, depth) => fixed_rows!(buffer, *k, *n, *depth, read_fixed_i8_slice),
            Leaf::I16Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_i16_slice)
            }
            Leaf::I32Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_i32_slice)
            }
            Leaf::I64Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_i64_slice)
            }
            Leaf::U8Fixed(k, n, depth) => fixed_rows!(buffer, *k, *n, *depth, read_fixed_u8_slice),
            Leaf::U16Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_u16_slice)
            }
            Leaf::U32Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_u32_slice)
            }
            Leaf::U64Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_u64_slice)
            }
            Leaf::F16Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_f16_slice)
            }
            Leaf::F32Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_f32_slice)
            }
            Leaf::F64Fixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_f64_slice)
            }
            Leaf::TimestampFixed(k, n, depth) => {
                fixed_rows!(buffer, *k, *n, *depth, read_fixed_time_slice)
            }
            Leaf::BoolFixed(k, n, depth) => {
                let bits = buffer.read_fixed_bool_slice(*n, *k)?;
                let arrays = bits.chunks_exact((*k).max(1)).map(PyroValue::from);
                fold_leaves(buffer, *n, *depth, arrays)?
            }
            Leaf::I8FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_i8_option_slice)
            }
            Leaf::I16FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_i16_option_slice)
            }
            Leaf::I32FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_i32_option_slice)
            }
            Leaf::I64FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_i64_option_slice)
            }
            Leaf::U8FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_u8_option_slice)
            }
            Leaf::U16FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_u16_option_slice)
            }
            Leaf::U32FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_u32_option_slice)
            }
            Leaf::U64FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_u64_option_slice)
            }
            Leaf::F16FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_f16_option_slice)
            }
            Leaf::F32FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_f32_option_slice)
            }
            Leaf::F64FixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_f64_option_slice)
            }
            Leaf::TimestampFixedOption(k, n, depth) => {
                fixed_option_rows!(buffer, *k, *n, *depth, read_fixed_time_option_slice)
            }
            Leaf::BoolFixedOption(k, n, depth) => {
                let (bits, option) = buffer.read_fixed_bool_option_slice(*n, *k)?;
                let mut present = 0;
                let samples = option.iter().by_vals().map(|some| {
                    if !some {
                        return PyroValue::Null;
                    }
                    let array = &bits[present * k..(present + 1) * k];
                    present += 1;
                    PyroValue::from(array)
                });
                fold_leaves(buffer, *n, *depth, samples)?
            }
        })
    }
}
