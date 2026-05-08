//! Test FromRow with nested structs
use pyroduct::format::{FromRow, PyroRow, PyroValue};
struct Address {
    street: String,
    city: String,
    zip: u32,
}
impl ::pyroduct::format::value::TypeableRow for Address {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "street",
                                <String as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <String as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "city",
                                <String as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <String as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "zip",
                                <u32 as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <u32 as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                    ]),
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
struct Person {
    name: String,
    age: i32,
    address: Vec<Address>,
}
impl ::pyroduct::format::value::TypeableRow for Person {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "name",
                                <String as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <String as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "age",
                                <i32 as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <i32 as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "address",
                                <Vec<
                                    Address,
                                > as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <Vec<
                                    Address,
                                > as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                    ]),
                ),
            ),
            documentation: None,
        }
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroRow<'a>> for Person {
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
                    match row.get("address").ok_or_else(|| "Missing field: address")? {
                        ::pyroduct::PyroValue::List(items) => {
                            items
                                .iter()
                                .map(|v| {
                                    v.clone()
                                        .try_into()
                                        .map_err(|_| "Failed to convert element in field 'address'")
                                })
                                .collect::<Result<Vec<Address>, _>>()?
                        }
                        _ => return Err("Expected List for field 'address'"),
                    }
                },
            })
        })();
        result.map_err(|_| row)
    }
}
impl<'a> std::convert::TryFrom<&::pyroduct::PyroRow<'a>> for Person {
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
                match row.get("address").ok_or_else(|| "Missing field: address")? {
                    ::pyroduct::PyroValue::List(items) => {
                        items
                            .iter()
                            .map(|v| {
                                v.clone()
                                    .try_into()
                                    .map_err(|_| "Failed to convert element in field 'address'")
                            })
                            .collect::<Result<Vec<Address>, _>>()?
                    }
                    _ => return Err("Expected List for field 'address'"),
                }
            },
        })
    }
}
impl<'a> std::convert::TryFrom<::pyroduct::PyroValue<'a>> for Person {
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
impl<'a> std::convert::TryFrom<&::pyroduct::PyroValue<'a>> for Person {
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
        (
            "address",
            PyroValue::List(
                <[_]>::into_vec(::alloc::boxed::box_new([PyroValue::Group(addr_row)])),
            ),
        ),
    ]);
    let p = Person::try_from(&person_row).unwrap();
    match (&p.name, &"Bob") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p.age, &30) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p.address.street, &"123 Main St") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p.address.city, &"Springfield") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p.address.zip, &12345) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
