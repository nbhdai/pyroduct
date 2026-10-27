use std::fmt;

use crate::{schema::PyroType, value::PyroValue};

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error(transparent)]
    Schema(#[from] pyro_schema::SchemaError),
    #[error("inode index {0} out of range (count: {1})")]
    InodeOutOfRange(u32, usize),
    #[error("raw data slice [{0}..{1}) out of range (len: {2})")]
    RawSliceOutOfRange(usize, usize, usize),
    #[error("alignment violation: expected {expected}-byte alignment at offset {offset}")]
    AlignmentViolation { expected: usize, offset: usize },
    #[error("root inode is not a Group")]
    RootNotGroup,
    #[error("type mismatch: writer expected {expected}, got {actual}")]
    TypeMismatch {
        expected: PyroType,
        actual: PyroType,
    },
    #[error("Insufficient bytes for the fixed portion of this type of message")]
    UnderSized { expected: usize, actual: usize },
    #[error("Writer was not prepared")]
    NotPrepared,
    #[error("Unable to make writer")]
    InvalidInodes,
    /// A string isn't valid UTF-8. `range` is the offending byte sequence's
    /// position within the view's byte blob (it runs to the end of the
    /// string when the input ended mid-character); `source` says why.
    #[error("invalid UTF-8 at bytes {}..{}: {source}", range.start, range.end)]
    Utf8 {
        range: std::ops::Range<usize>,
        source: std::str::Utf8Error,
    },
    /// A column holds a different number of elements than the wire encodes
    /// for it: a group's field against the group's row count (only present
    /// rows, for a nullable group), or a map's keys or values against its
    /// entry count. `index` is the column — the field's position within the
    /// group, or `0` for a map's keys and `1` for its values.
    #[error("corrupt data: column {index} holds {have} elements, expected {expected}")]
    LengthMismatch {
        expected: usize,
        have: usize,
        index: usize,
    },
    /// A nested-list topology's concatenated words don't start the next
    /// level where one was expected — `word` is the position within the
    /// topology's own word run (not a buffer offset), `found` the value read
    /// there instead of [`crate::wire::nesting::TOPOLOGY_RESET`].
    #[error(
        "corrupt topology: word {word} was expected to start a level (u32::MAX) but is {found}"
    )]
    InvalidTopologySentinel { word: usize, found: usize },
    /// A level's sentinel is followed straight by another sentinel (or the
    /// end of the words): `level` holds no boundaries. Writing stops at the
    /// first level that ends in 0, so an empty level is never written.
    /// `word` is the sentinel's position within the topology's word run.
    #[error("corrupt topology: level {level} at word {word} is empty")]
    InvalidTopologyEmptyLevel { level: usize, word: usize },
    /// A topology's data holds more levels than the schema's nesting depth
    /// (`max`) admits.
    #[error("corrupt topology: found more than {max} levels")]
    InvalidTopologyDepth { max: usize },
    /// The topology's length disagrees with the data length.
    #[error("corrupt topology: topology length {topo} does not fit data {data}")]
    InvalidTopologyLen { topo: usize, data: usize },

    #[error("corrupt option: option length {valid} does not fit data {data}")]
    InvalidOptionLen { valid: usize, data: usize },
    /// A run of fixed-size arrays doesn't divide into whole arrays of
    /// `expected` elements.
    #[error("corrupt fixed-size arrays: run is not a whole number of {expected}-element arrays")]
    InvalidFixedLen { expected: usize },
    /// A level's cumulative boundaries go backwards: the boundary at `index`
    /// within `level` (the level's leading sentinel is index 0) is `found`,
    /// below the `previous` one.
    #[error(
        "corrupt topology: level {level} boundary {index} is {found}, below the previous {previous}"
    )]
    InvalidTopologyOrder {
        level: usize,
        index: usize,
        previous: usize,
        found: usize,
    },
    /// A level's final boundary (`last`, its total count of elements one
    /// level down) is not the number of lists the next level holds.
    #[error(
        "corrupt topology: level {level} ends at {last} but level {} holds {next} lists",
        level + 1
    )]
    InvalidTopologyLevel {
        level: usize,
        last: usize,
        next: usize,
    },
    /// A fixed-capacity [`crate::buf::PyroSink`] ran out of room. `needed` is
    /// the total length the write would have produced, not the size of the
    /// failing write alone.
    ///
    /// This should be a *bug report*, not a runtime condition on the happy
    /// path: the io layer reserves `encoded_len()` exactly, so hitting this
    /// means the writer's length computation and its write disagree.
    #[error(
        "sink capacity exceeded: write needed {needed} bytes, sink capacity is {capacity} \
         (this is a bug: the writer's length computation and its write disagree)"
    )]
    CapacityExceeded { needed: usize, capacity: usize },
    /// The message runs to `end` bytes, past what the structure word can
    /// address: every offset must be under `max`, the word's `RESET`
    /// (which is the topology's tier sentinel, so never an offset itself).
    /// A `u16` message this big needs `#[pyro(Word = u32)]`.
    #[error("message is {end} bytes, past the structure word's offset limit of {max}")]
    OffsetOverflow { end: usize, max: usize },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InvalidValue<'a> {
    #[serde(borrow)]
    pub value: Option<PyroValue<'a>>,
    pub v_type: PyroType,
}

impl<'a> InvalidValue<'a> {
    /// Creates a new InvalidValue, preserving the data by converting it to 'static.
    /// Use this for small scalar values that are cheap to clone and store.
    pub fn new(value: PyroValue<'a>) -> Self {
        Self {
            v_type: value.datatype(),
            value: Some(value),
        }
    }

    /// Creates a new InvalidValue WITHOUT the data (storing only the type).
    /// Use this for large structures (Groups, huge Lists, Strings) where storing
    /// the invalid data would be too expensive or redundant.
    pub fn new_large(value: PyroValue<'a>) -> Self {
        Self {
            v_type: value.datatype(),
            value: None,
        }
    }

    pub fn to_static(self) -> InvalidValue<'static> {
        InvalidValue {
            value: self.value.map(|v| v.to_static()),
            v_type: self.v_type,
        }
    }
}

impl<'a> fmt::Display for InvalidValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            // Case 1: Value was preserved (small scalar)
            Some(v) => write!(f, "Invalid(Type: {}, Value: {})", self.v_type, v),

            // Case 2: Value was omitted (large structure)
            None => write!(f, "Invalid(Type: {})", self.v_type),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValueError<'a> {
    #[error("Invalid Time")]
    Time,
    #[error("Invalid Scalar")]
    InvalidScalar(Box<InvalidValue<'a>>),
    #[error("Can't cast {0:?} to {1}")]
    Cast(Box<InvalidValue<'a>>, PyroType),

    #[cfg(feature = "arrow")]
    #[error("Arrow Error")]
    ArrowError(#[from] ::arrow::error::ArrowError),

    #[error("Method `{0}` is not available for type `{1}`")]
    Unimplemented(String, String),
}

impl<'a> ValueError<'a> {
    pub fn invalid(value: impl Into<PyroValue<'a>>) -> Self {
        ValueError::InvalidScalar(InvalidValue::new(value.into()).into())
    }
    pub fn invalid_large(value: impl Into<PyroValue<'a>>) -> Self {
        ValueError::InvalidScalar(InvalidValue::new_large(value.into()).into())
    }
    pub fn cast(value: impl Into<PyroValue<'a>>, target: &PyroType) -> Self {
        ValueError::Cast(InvalidValue::new(value.into()).into(), target.clone())
    }
    pub fn cast_large(value: impl Into<PyroValue<'a>>, target: &PyroType) -> Self {
        ValueError::Cast(InvalidValue::new_large(value.into()).into(), target.clone())
    }

    pub fn to_static(self) -> ValueError<'static> {
        match self {
            ValueError::Time => ValueError::Time,
            ValueError::InvalidScalar(v) => ValueError::InvalidScalar(v.to_static().into()),
            ValueError::Cast(v, t) => ValueError::Cast(v.to_static().into(), t),
            #[cfg(feature = "arrow")]
            ValueError::ArrowError(e) => ValueError::ArrowError(e),
            ValueError::Unimplemented(s, t) => ValueError::Unimplemented(s, t),
        }
    }
}

pub type ValueResult<'a, A> = std::result::Result<A, ValueError<'a>>;

impl PartialEq for ValueError<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unimplemented(l0, l1), Self::Unimplemented(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::InvalidScalar(l0), Self::InvalidScalar(r0)) => l0 == r0,
            // (Self::ArrowError(_), Self::ArrowError(_)) => true,
            (Self::Cast(l0, l1), Self::Cast(r0, r1)) => l0 == r0 && l1 == r1,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
