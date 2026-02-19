//! Test FromRow with nested structs
use pyroduct::{FromRow, PyroValue, PyroRow};
struct Address {
    street: String,
    city: String,
    zip: u32,
}
impl ::pyroduct::value::TypeableRow for Address {
    fn schema() -> ::pyroduct::value::PyroSchema<'static> {
        ::pyroduct::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "street",
                                    <String as ::pyroduct::value::Typeable>::pyro_type(),
                                    <String as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "city",
                                    <String as ::pyroduct::value::Typeable>::pyro_type(),
                                    <String as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "zip",
                                    <u32 as ::pyroduct::value::Typeable>::pyro_type(),
                                    <u32 as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                        ],
                    ),
                ),
            ),
            documentation: None,
        }
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroRow<'a>> for Address {
    type Error = ::pyroduct::PyroRow<'a>;
    fn try_from(row: ::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        let result = (|| -> Result<Self, &'static str> {
            Ok(Self {
                street: {
                    let val = row
                        .get("street")
                        .ok_or_else(|| "Missing field: street")?
                        .clone();
                    val.try_into().map_err(|_| "Failed to convert field 'street'")?
                },
                city: {
                    let val = row
                        .get("city")
                        .ok_or_else(|| "Missing field: city")?
                        .clone();
                    val.try_into().map_err(|_| "Failed to convert field 'city'")?
                },
                zip: {
                    let val = row
                        .get("zip")
                        .ok_or_else(|| "Missing field: zip")?
                        .clone();
                    val.try_into().map_err(|_| "Failed to convert field 'zip'")?
                },
            })
        })();
        result.map_err(|_| row)
    }
}
impl<'a> std::convert::TryFrom<&::pyroduct::PyroRow<'a>> for Address {
    type Error = &'static str;
    fn try_from(row: &::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            street: {
                let val = row
                    .get("street")
                    .ok_or_else(|| "Missing field: street")?
                    .clone();
                val.try_into().map_err(|_| "Failed to convert field 'street'")?
            },
            city: {
                let val = row.get("city").ok_or_else(|| "Missing field: city")?.clone();
                val.try_into().map_err(|_| "Failed to convert field 'city'")?
            },
            zip: {
                let val = row.get("zip").ok_or_else(|| "Missing field: zip")?.clone();
                val.try_into().map_err(|_| "Failed to convert field 'zip'")?
            },
        })
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroValue<'a>> for Address {
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
impl<'a> std::convert::TryFrom<&::pyroduct::PyroValue<'a>> for Address {
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
struct Person<T> {
    name: String,
    age: i32,
    address: T,
}
impl<T: ::pyroduct::value::Typeable> ::pyroduct::value::TypeableRow for Person<T> {
    fn schema() -> ::pyroduct::value::PyroSchema<'static> {
        ::pyroduct::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "name",
                                    <String as ::pyroduct::value::Typeable>::pyro_type(),
                                    <String as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "age",
                                    <i32 as ::pyroduct::value::Typeable>::pyro_type(),
                                    <i32 as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "address",
                                    <T as ::pyroduct::value::Typeable>::pyro_type(),
                                    <T as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                        ],
                    ),
                ),
            ),
            documentation: None,
        }
    }
}
impl<
    'a,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<::pyroduct::PyroRow<'a>> for Person<T> {
    type Error = ::pyroduct::PyroRow<'a>;
    fn try_from(row: ::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        let result = (|| -> Result<Self, &'static str> {
            Ok(Self {
                name: {
                    let val = row
                        .get("name")
                        .ok_or_else(|| "Missing field: name")?
                        .clone();
                    val.try_into().map_err(|_| "Failed to convert field 'name'")?
                },
                age: {
                    let val = row
                        .get("age")
                        .ok_or_else(|| "Missing field: age")?
                        .clone();
                    val.try_into().map_err(|_| "Failed to convert field 'age'")?
                },
                address: {
                    let val = row
                        .get("address")
                        .ok_or_else(|| "Missing field: address")?
                        .clone();
                    val.try_into().map_err(|_| "Failed to convert field 'address'")?
                },
            })
        })();
        result.map_err(|_| row)
    }
}
impl<
    'a,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<&::pyroduct::PyroRow<'a>> for Person<T> {
    type Error = &'static str;
    fn try_from(row: &::pyroduct::PyroRow<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            name: {
                let val = row.get("name").ok_or_else(|| "Missing field: name")?.clone();
                val.try_into().map_err(|_| "Failed to convert field 'name'")?
            },
            age: {
                let val = row.get("age").ok_or_else(|| "Missing field: age")?.clone();
                val.try_into().map_err(|_| "Failed to convert field 'age'")?
            },
            address: {
                let val = row
                    .get("address")
                    .ok_or_else(|| "Missing field: address")?
                    .clone();
                val.try_into().map_err(|_| "Failed to convert field 'address'")?
            },
        })
    }
}
impl<
    'a,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<::pyroduct::PyroValue<'a>> for Person<T> {
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
impl<
    'a,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<&::pyroduct::PyroValue<'a>> for Person<T> {
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
fn main() {
    let addr_row = PyroRow::from([
        ("street", PyroValue::from("123 Main St")),
        ("city", PyroValue::from("Springfield")),
        ("zip", PyroValue::U32(12345)),
    ]);
    let person_row = PyroRow::from([
        ("name", PyroValue::from("Bob")),
        ("age", PyroValue::I32(30)),
        ("address", PyroValue::Group(addr_row)),
    ]);
    let p = Person::<Address>::try_from(&person_row).unwrap();
}
