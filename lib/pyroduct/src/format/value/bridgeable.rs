use std::borrow::Cow;
use std::panic::Location;

use crate::format::bridgeable::{Decoder, Encoder};
use crate::format::header::{DataStatus, PyroHeader};
use crate::format::value::{PrimitiveValueList, PyroValue, Time};
use crate::format::{Bridgeable, PyroVec, PyroView};
use crate::{CapturedError, PyroError};

// =============================================================================
// Macros for Bridgeable Implementation
// =============================================================================

macro_rules! impl_bridgeable_scalar {
    ($t:ty, $variant:ident, $encoder:ident, $decoder:ident) => {
        pub struct $encoder;
        impl Default for $encoder {
            fn default() -> Self {
                Self
            }
        }

        impl Encoder<$t> for $encoder {
            fn encode(&mut self, value: &$t) -> Result<PyroVec, PyroError> {
                PyroValue::$variant(*value).to_wire()
            }
        }

        impl Encoder<&$t> for $encoder {
            fn encode(&mut self, value: &&$t) -> Result<PyroVec, PyroError> {
                PyroValue::$variant(**value).to_wire()
            }
        }

        pub struct $decoder;
        impl Default for $decoder {
            fn default() -> Self {
                Self
            }
        }
        impl<'a> Decoder<'a, $t> for $decoder {
            fn decode(&mut self, view: PyroView<'a>) -> Result<$t, PyroError> {
                let val = PyroValue::parse_wire(view)?;
                if let PyroValue::$variant(inner) = val {
                    Ok(inner)
                } else {
                    Err(PyroError::deserialization(Box::new(
                        CapturedError::new(format!(
                            "Expected {}, found {:?}",
                            stringify!($variant),
                            val
                        ))
                        .with_location(Location::caller()),
                    )))
                }
            }
        }

        impl Bridgeable for $t {
            type Encoder = $encoder;
            type Decoder = $decoder;
            type Ref<'a> = $t;
        }
    };
}

impl_bridgeable_scalar!(bool, Bool, BoolEncoder, BoolDecoder);
impl_bridgeable_scalar!(i8, I8, I8Encoder, I8Decoder);
impl_bridgeable_scalar!(i16, I16, I16Encoder, I16Decoder);
impl_bridgeable_scalar!(i32, I32, I32Encoder, I32Decoder);
impl_bridgeable_scalar!(i64, I64, I64Encoder, I64Decoder);
impl_bridgeable_scalar!(u8, U8, U8Encoder, U8Decoder);
impl_bridgeable_scalar!(u16, U16, U16Encoder, U16Decoder);
impl_bridgeable_scalar!(u32, U32, U32Encoder, U32Decoder);
impl_bridgeable_scalar!(u64, U64, U64Encoder, U64Decoder);
impl_bridgeable_scalar!(half::f16, F16, F16Encoder, F16Decoder);
impl_bridgeable_scalar!(f32, F32, F32Encoder, F32Decoder);
impl_bridgeable_scalar!(f64, F64, F64Encoder, F64Decoder);
impl_bridgeable_scalar!(Time, Timestamp, TimestampEncoder, TimestampDecoder);

// --- String ---

pub struct StringEncoder;
impl Default for StringEncoder {
    fn default() -> Self {
        Self
    }
}
impl Encoder<&str> for StringEncoder {
    fn encode(&mut self, value: &&str) -> Result<PyroVec, PyroError> {
        PyroValue::Str(Cow::Borrowed(value)).to_wire()
    }
}

impl Encoder<String> for StringEncoder {
    fn encode(&mut self, value: &String) -> Result<PyroVec, PyroError> {
        PyroValue::Str(Cow::Borrowed(value.as_str())).to_wire()
    }
}

pub struct StringDecoder;
impl Default for StringDecoder {
    fn default() -> Self {
        Self
    }
}
impl<'a> Decoder<'a, &'a str> for StringDecoder {
    fn decode(&mut self, view: PyroView<'a>) -> Result<&'a str, PyroError> {
        let val = PyroValue::parse_wire(view)?;
        if let PyroValue::Str(cow) = val {
            Ok(match cow {
                Cow::Borrowed(s) => s,
                Cow::Owned(_) => unreachable!("rkyv parsing should return borrowed data"),
            })
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str, found {:?}", val))
                    .with_location(Location::caller()),
            )))
        }
    }
}

impl<'a> Decoder<'a, String> for StringDecoder {
    fn decode(&mut self, view: PyroView<'a>) -> Result<String, PyroError> {
        let val = PyroValue::parse_wire(view)?;
        if let PyroValue::Str(cow) = val {
            Ok(match cow {
                Cow::Borrowed(s) => s.to_owned(),
                Cow::Owned(s) => s,
            })
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str, found {:?}", val))
                    .with_location(Location::caller()),
            )))
        }
    }
}

impl Bridgeable for String {
    type Encoder = StringEncoder;
    type Decoder = StringDecoder;
    type Ref<'a> = &'a str;
}

impl Bridgeable for &str {
    type Encoder = StringEncoder;
    type Decoder = StringDecoder;
    type Ref<'a> = &'a str;
}

// --- Primitive Lists ---

macro_rules! impl_bridgeable_list {
    ($t:ty, $variant:ident, $encoder:ident, $decoder:ident) => {
        pub struct $encoder;
        impl Default for $encoder {
            fn default() -> Self {
                Self
            }
        }
        impl Encoder<&[$t]> for $encoder {
            fn encode(&mut self, value: &&[$t]) -> Result<PyroVec, PyroError> {
                PyroValue::PrimitiveList(PrimitiveValueList::$variant(Cow::Borrowed(*value)))
                    .to_wire()
            }
        }
        impl Encoder<Vec<$t>> for $encoder {
            fn encode(&mut self, value: &Vec<$t>) -> Result<PyroVec, PyroError> {
                PyroValue::PrimitiveList(PrimitiveValueList::$variant(Cow::Borrowed(
                    value.as_slice(),
                )))
                .to_wire()
            }
        }

        #[derive(Default)]
        pub struct $decoder;
        impl<'a> Decoder<'a, &'a [$t]> for $decoder {
            fn decode(&mut self, view: PyroView<'a>) -> Result<&'a [$t], PyroError> {
                let val = PyroValue::parse_wire(view)?;
                if let PyroValue::PrimitiveList(PrimitiveValueList::$variant(cow)) = val {
                    Ok(match cow {
                        Cow::Borrowed(s) => s,
                        Cow::Owned(_) => unreachable!("rkyv parsing should return borrowed data"),
                    })
                } else {
                    Err(PyroError::deserialization(Box::new(
                        CapturedError::new(format!(
                            "Expected PrimitiveList({}), found {:?}",
                            stringify!($variant),
                            val
                        ))
                        .with_location(Location::caller()),
                    )))
                }
            }
        }

        impl Bridgeable for Vec<$t> {
            type Encoder = $encoder;
            type Decoder = $decoder;
            type Ref<'a> = &'a [$t];
        }
    };
}

impl_bridgeable_list!(bool, Bool, BoolListEncoder, BoolListDecoder);
impl_bridgeable_list!(i8, I8, I8ListEncoder, I8ListDecoder);
impl_bridgeable_list!(i16, I16, I16ListEncoder, I16ListDecoder);
impl_bridgeable_list!(i32, I32, I32ListEncoder, I32ListDecoder);
impl_bridgeable_list!(i64, I64, I64ListEncoder, I64ListDecoder);
impl_bridgeable_list!(u8, U8, U8ListEncoder, U8ListDecoder);
impl_bridgeable_list!(u16, U16, U16ListEncoder, U16ListDecoder);
impl_bridgeable_list!(u32, U32, U32ListEncoder, U32ListDecoder);
impl_bridgeable_list!(u64, U64, U64ListEncoder, U64ListDecoder);
impl_bridgeable_list!(half::f16, F16, F16ListEncoder, F16ListDecoder);
impl_bridgeable_list!(f32, F32, F32ListEncoder, F32ListDecoder);
impl_bridgeable_list!(f64, F64, F64ListEncoder, F64ListDecoder);

pub struct EmptyEncoder;
impl Default for EmptyEncoder {
    fn default() -> Self {
        Self
    }
}

impl Encoder<()> for EmptyEncoder {
    fn encode(&mut self, _value: &()) -> Result<PyroVec, PyroError> {
        Ok(PyroVec::ok())
    }
}

pub struct EmptyDecoder;
impl Default for EmptyDecoder {
    fn default() -> Self {
        Self
    }
}
impl<'a> Decoder<'a, ()> for EmptyDecoder {
    fn decode(&mut self, view: PyroView<'a>) -> Result<(), PyroError> {
        if matches!(view.status(), Ok(DataStatus::Empty)) {
            Ok(())
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Ok, found {:?}", view))
                    .with_location(Location::caller()),
            )))
        }
    }
}

impl Bridgeable for () {
    type Encoder = EmptyEncoder;
    type Decoder = EmptyDecoder;
    type Ref<'a> = ();
}
