//! Test FromRow with empty struct (edge case)
use pyroduct::format::{DeepRef, FromRow, PyroRow, RefFromRow};
struct Empty {}
impl ::pyroduct::format::value::TypeableRow for Empty {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(::alloc::vec::Vec::new()),
            documentation: None,
        }
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroRow<'a>> for Empty {
    type Error = ::pyroduct::PyroRow<'a>;
    fn try_from(row: ::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        let result = (|| -> Result<Self, &'static str> { Ok(Self {}) })();
        result.map_err(|_| row)
    }
}
impl<'a> std::convert::TryFrom<&::pyroduct::PyroRow<'a>> for Empty {
    type Error = &'static str;
    fn try_from(row: &::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroValue<'a>> for Empty {
    type Error = ::pyroduct::PyroValue<'a>;
    fn try_from(value: ::pyroduct::PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            ::pyroduct::PyroValue::Group(r) => {
                match <Self as std::convert::TryFrom<
                    ::pyroduct::PyroRow<'a>,
                >>::try_from(r) {
                    Ok(s) => Ok(s),
                    Err(r) => Err(::pyroduct::PyroValue::Group(r)),
                }
            }
            v => Err(v),
        }
    }
}
impl<'a> std::convert::TryFrom<&::pyroduct::PyroValue<'a>> for Empty {
    type Error = &'static str;
    fn try_from(value: &::pyroduct::PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            ::pyroduct::PyroValue::Group(r) => {
                <Self as std::convert::TryFrom<&::pyroduct::PyroRow<'a>>>::try_from(r)
            }
            _ => Err("Expected Group"),
        }
    }
}
impl ::pyroduct::format::value::TypeableRow for EmptyRef<'_> {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(::alloc::vec::Vec::new()),
            documentation: None,
        }
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroRow<'a>> for EmptyRef<'a> {
    type Error = ::pyroduct::PyroRow<'a>;
    fn try_from(row: ::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        let result = (|| -> Result<Self, &'static str> {
            Ok(Self {
                _phantom: std::marker::PhantomData,
            })
        })();
        result.map_err(|_| row)
    }
}
impl<'a> std::convert::TryFrom<&::pyroduct::PyroRow<'a>> for EmptyRef<'a> {
    type Error = &'static str;
    fn try_from(row: &::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            _phantom: std::marker::PhantomData,
        })
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroValue<'a>> for EmptyRef<'a> {
    type Error = ::pyroduct::PyroValue<'a>;
    fn try_from(value: ::pyroduct::PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            ::pyroduct::PyroValue::Group(r) => {
                match <Self as std::convert::TryFrom<
                    ::pyroduct::PyroRow<'a>,
                >>::try_from(r) {
                    Ok(s) => Ok(s),
                    Err(r) => Err(::pyroduct::PyroValue::Group(r)),
                }
            }
            v => Err(v),
        }
    }
}
impl<'a> std::convert::TryFrom<&::pyroduct::PyroValue<'a>> for EmptyRef<'a> {
    type Error = &'static str;
    fn try_from(value: &::pyroduct::PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            ::pyroduct::PyroValue::Group(r) => {
                <Self as std::convert::TryFrom<&::pyroduct::PyroRow<'a>>>::try_from(r)
            }
            _ => Err("Expected Group"),
        }
    }
}
pub struct EmptyRef<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}
impl ::pyroduct::format::DeepRef for Empty {
    type Ref<'a> = EmptyRef<'a>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        EmptyRef {
            _phantom: std::marker::PhantomData,
        }
    }
}
fn main() {
    let row = PyroRow::new();
    let _e = EmptyRef::try_from(&row).unwrap();
}
