//! Test FromRow with nested structs
use pyroduct::{FromRow, PyroValue, PyroRow};
struct Address {
    street: String,
    city: String,
    zip: u32,
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
struct Person<S, T> {
    name: S,
    age: i32,
    address: T,
}
impl<
    'a,
    S: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<::pyroduct::PyroRow<'a>> for Person<S, T> {
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
    S: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<&::pyroduct::PyroRow<'a>> for Person<S, T> {
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
    S: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<::pyroduct::PyroValue<'a>> for Person<S, T> {
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
    S: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
    T: std::convert::TryFrom<::pyroduct::PyroValue<'a>>,
> std::convert::TryFrom<&::pyroduct::PyroValue<'a>> for Person<S, T> {
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
    let _p = Person::<String, Address>::try_from(&person_row).unwrap();
}
