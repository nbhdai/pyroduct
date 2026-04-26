use std::borrow::Cow;
use std::panic::Location;

use crate::format::bridgeable::{TypedBuf, TypedView};
use crate::format::value::{PrimitiveValueList, PyroRow, PyroValue, Time};
use crate::format::{Bridgeable, PyroVec, PyroView};
use crate::{CapturedError, PyroError};

// =============================================================================
// Macros for Bridgeable Implementation
// =============================================================================

macro_rules! impl_bridgeable_scalar {
    ($t:ty, $variant:ident) => {
        impl Bridgeable for $t {
            type Ref<'a> = $t;

            fn ship(&self) -> Result<PyroVec, PyroError> {
                let val = PyroValue::from(*self);
                val.to_wire()
            }

            #[track_caller]
            fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
                let val = PyroValue::parse_wire(vec.view())?;
                if let PyroValue::$variant(inner) = val {
                    Ok(TypedBuf { vec, inner })
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

            fn expose_view<'a>(
                view: PyroView<'a>,
            ) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
                let val = PyroValue::parse_wire(view)?;
                if let PyroValue::$variant(inner) = val {
                    Ok(TypedView { view, inner })
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
    };
}

impl_bridgeable_scalar!(bool, Bool);
impl_bridgeable_scalar!(i8, I8);
impl_bridgeable_scalar!(i16, I16);
impl_bridgeable_scalar!(i32, I32);
impl_bridgeable_scalar!(i64, I64);
impl_bridgeable_scalar!(u8, U8);
impl_bridgeable_scalar!(u16, U16);
impl_bridgeable_scalar!(u32, U32);
impl_bridgeable_scalar!(u64, U64);
impl_bridgeable_scalar!(half::f16, F16);
impl_bridgeable_scalar!(f32, F32);
impl_bridgeable_scalar!(f64, F64);
impl_bridgeable_scalar!(Time, Timestamp);

// --- String ---

impl Bridgeable for String {
    type Ref<'a> = &'a str;

    fn ship(&self) -> Result<PyroVec, PyroError> {
        let val = PyroValue::from(self);
        val.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let val = PyroValue::parse_wire(vec.view())?;
        if let PyroValue::Str(cow) = val {
            // SAFETY: We own the PyroVec, and the string borrows from it.
            let s = cow.as_ref();
            let extended = unsafe { std::mem::transmute::<&str, &'static str>(s) };
            Ok(TypedBuf {
                vec,
                inner: extended,
            })
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str, found {:?}", val))
                    .with_location(Location::caller()),
            )))
        }
    }

    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let val = PyroValue::parse_wire(view)?;
        if let PyroValue::Str(cow) = val {
            let s = match cow {
                Cow::Borrowed(s) => s,
                Cow::Owned(_) => unreachable!("rkyv parsing should return borrowed data"),
            };
            Ok(TypedView { view, inner: s })
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str, found {:?}", val))
                    .with_location(Location::caller()),
            )))
        }
    }
}

// --- Option ---

impl Bridgeable for Option<String> {
    type Ref<'a> = Option<&'a str>;

    fn ship(&self) -> Result<PyroVec, PyroError> {
        let val = PyroValue::from(self);
        val.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let val = PyroValue::parse_wire(vec.view())?;
        match val {
            PyroValue::Str(cow) => {
                let s = cow.as_ref();
                let extended = unsafe { std::mem::transmute::<&str, &'static str>(s) };
                Ok(TypedBuf {
                    vec,
                    inner: Some(extended),
                })
            }
            PyroValue::Null => Ok(TypedBuf { vec, inner: None }),
            _ => Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str or Null, found {:?}", val))
                    .with_location(Location::caller()),
            ))),
        }
    }

    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let val = PyroValue::parse_wire(view)?;
        match val {
            PyroValue::Str(cow) => {
                let s = match cow {
                    Cow::Borrowed(s) => s,
                    Cow::Owned(_) => unreachable!("rkyv parsing should return borrowed data"),
                };
                Ok(TypedView {
                    view,
                    inner: Some(s),
                })
            }
            PyroValue::Null => Ok(TypedView { view, inner: None }),
            _ => Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str or Null, found {:?}", val))
                    .with_location(Location::caller()),
            ))),
        }
    }
}

macro_rules! impl_bridgeable_option_scalar {
    ($t:ty, $variant:ident) => {
        impl Bridgeable for Option<$t> {
            type Ref<'a> = Option<$t>;

            fn ship(&self) -> Result<PyroVec, PyroError> {
                let val = PyroValue::from(*self);
                val.to_wire()
            }

            #[track_caller]
            fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
                let val = PyroValue::parse_wire(vec.view())?;
                match val {
                    PyroValue::$variant(inner) => Ok(TypedBuf {
                        vec,
                        inner: Some(inner),
                    }),
                    PyroValue::Null => Ok(TypedBuf { vec, inner: None }),
                    _ => Err(PyroError::deserialization(Box::new(
                        CapturedError::new(format!(
                            "Expected {} or Null, found {:?}",
                            stringify!($variant),
                            val
                        ))
                        .with_location(Location::caller()),
                    ))),
                }
            }

            fn expose_view<'a>(
                view: PyroView<'a>,
            ) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
                let val = PyroValue::parse_wire(view)?;
                match val {
                    PyroValue::$variant(inner) => Ok(TypedView {
                        view,
                        inner: Some(inner),
                    }),
                    PyroValue::Null => Ok(TypedView { view, inner: None }),
                    _ => Err(PyroError::deserialization(Box::new(
                        CapturedError::new(format!(
                            "Expected {} or Null, found {:?}",
                            stringify!($variant),
                            val
                        ))
                        .with_location(Location::caller()),
                    ))),
                }
            }
        }
    };
}

impl_bridgeable_option_scalar!(bool, Bool);
impl_bridgeable_option_scalar!(i8, I8);
impl_bridgeable_option_scalar!(i16, I16);
impl_bridgeable_option_scalar!(i32, I32);
impl_bridgeable_option_scalar!(i64, I64);
impl_bridgeable_option_scalar!(u8, U8);
impl_bridgeable_option_scalar!(u16, U16);
impl_bridgeable_option_scalar!(u32, U32);
impl_bridgeable_option_scalar!(u64, U64);
impl_bridgeable_option_scalar!(half::f16, F16);
impl_bridgeable_option_scalar!(f32, F32);
impl_bridgeable_option_scalar!(f64, F64);
impl_bridgeable_option_scalar!(Time, Timestamp);

impl Bridgeable for &str {
    type Ref<'a> = &'a str;

    fn ship(&self) -> Result<PyroVec, PyroError> {
        let val = PyroValue::from(*self);
        val.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let val = PyroValue::parse_wire(vec.view())?;
        if let PyroValue::Str(cow) = val {
            let s = cow.as_ref();
            let extended = unsafe { std::mem::transmute::<&str, &'static str>(s) };
            Ok(TypedBuf {
                vec,
                inner: extended,
            })
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str, found {:?}", val))
                    .with_location(Location::caller()),
            )))
        }
    }

    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let val = PyroValue::parse_wire(view)?;
        if let PyroValue::Str(cow) = val {
            // cow is Cow<'a, str> because it was parsed from view which has lifetime 'a
            // Wait, cow is actually Cow<'a, str> because parse_wire returns PyroValue<'a>.
            // However, the 'a in PyroValue<'a> is the lifetime of the view.

            // Let's check the signature of expose_view:
            // fn expose_view<'a>(vec: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError>

            // Self::Ref<'a> is &'a str.

            let s = match cow {
                Cow::Borrowed(s) => s,
                Cow::Owned(_) => unreachable!("rkyv parsing should return borrowed data"),
            };
            Ok(TypedView { view, inner: s })
        } else {
            Err(PyroError::deserialization(Box::new(
                CapturedError::new(format!("Expected Str, found {:?}", val))
                    .with_location(Location::caller()),
            )))
        }
    }
}

// --- PyroValue ---

impl Bridgeable for PyroValue<'static> {
    type Ref<'a> = PyroValue<'a>;

    fn ship(&self) -> Result<PyroVec, PyroError> {
        self.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let val = PyroValue::parse_wire(vec.view())?;
        let extended = unsafe { std::mem::transmute::<PyroValue<'_>, PyroValue<'static>>(val) };
        Ok(TypedBuf {
            vec,
            inner: extended,
        })
    }

    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let val = PyroValue::parse_wire(view)?;
        Ok(TypedView { view, inner: val })
    }
}

// --- PyroRow ---

impl Bridgeable for PyroRow<'static> {
    type Ref<'a> = PyroRow<'a>;

    fn ship(&self) -> Result<PyroVec, PyroError> {
        self.to_wire()
    }

    #[track_caller]
    fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
        let val = PyroRow::parse_wire(vec.view())?;
        let extended = unsafe { std::mem::transmute::<PyroRow<'_>, PyroRow<'static>>(val) };
        Ok(TypedBuf {
            vec,
            inner: extended,
        })
    }

    fn expose_view<'a>(view: PyroView<'a>) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
        let val = PyroRow::parse_wire(view)?;
        Ok(TypedView { view, inner: val })
    }
}

// --- Primitive Lists ---

macro_rules! impl_bridgeable_list {
    ($t:ty, $variant:ident) => {
        impl Bridgeable for Vec<$t> {
            type Ref<'a> = &'a [$t];

            fn ship(&self) -> Result<PyroVec, PyroError> {
                let val = PyroValue::from(self.as_slice());
                val.to_wire()
            }

            #[track_caller]
            fn expose(vec: PyroVec) -> Result<TypedBuf<Self::Ref<'static>>, PyroError> {
                let val = PyroValue::parse_wire(vec.view())?;
                if let PyroValue::PrimitiveList(PrimitiveValueList::$variant(cow)) = val {
                    let s = cow.as_ref();
                    let extended = unsafe { std::mem::transmute::<&[$t], &'static [$t]>(s) };
                    Ok(TypedBuf {
                        vec,
                        inner: extended,
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

            fn expose_view<'a>(
                view: PyroView<'a>,
            ) -> Result<TypedView<'a, Self::Ref<'a>>, PyroError> {
                let val = PyroValue::parse_wire(view)?;
                if let PyroValue::PrimitiveList(PrimitiveValueList::$variant(cow)) = val {
                    let s = match cow {
                        Cow::Borrowed(s) => s,
                        Cow::Owned(_) => unreachable!("rkyv parsing should return borrowed data"),
                    };
                    Ok(TypedView { view, inner: s })
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
    };
}

impl_bridgeable_list!(bool, Bool);
impl_bridgeable_list!(i8, I8);
impl_bridgeable_list!(i16, I16);
impl_bridgeable_list!(i32, I32);
impl_bridgeable_list!(i64, I64);
impl_bridgeable_list!(u8, U8);
impl_bridgeable_list!(u16, U16);
impl_bridgeable_list!(u32, U32);
impl_bridgeable_list!(u64, U64);
impl_bridgeable_list!(half::f16, F16);
impl_bridgeable_list!(f32, F32);
impl_bridgeable_list!(f64, F64);
