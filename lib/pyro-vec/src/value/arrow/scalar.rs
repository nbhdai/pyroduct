use crate::value::time::Time;
use arrow::array::*;
use arrow::datatypes::*;
use std::borrow::Cow;
use std::ops::Deref;

use crate::value::{PyroRow, PyroValue, PrimitiveValueList, Result, ValueError};

pub trait ScalarValuable {
    /// Gets a reference to the value at the given index.
    /// This does not copy string, bytes, or primitive non-null lists (uses Cow::Borrowed).
    fn scalar<'a>(&'a self, i: usize) -> Result<PyroValue<'a>>;
}

fn convert_u64_time(val: i128, unit: TimeUnit) -> Time {
    Time(match unit {
        TimeUnit::Second => val * 1_000_000_000,
        TimeUnit::Millisecond => val * 1_000_000,
        TimeUnit::Microsecond => val * 1_000,
        TimeUnit::Nanosecond => val,
    })
}

impl<T: Array> ScalarValuable for T {
    fn scalar<'a>(&'a self, i: usize) -> Result<PyroValue<'a>> {
        if self.is_null(i) {
            return Ok(PyroValue::Null);
        }
        let value = match self.data_type() {
            DataType::Null => unreachable!(),
            DataType::Int8 => {
                let array = as_primitive_array::<Int8Type>(self);
                PyroValue::I8(array.value(i))
            }
            DataType::Int16 => {
                let array = as_primitive_array::<Int16Type>(self);
                PyroValue::I16(array.value(i))
            }
            DataType::Int32 => {
                let array = as_primitive_array::<Int32Type>(self);
                PyroValue::I32(array.value(i))
            }
            DataType::Int64 => {
                let array = as_primitive_array::<Int64Type>(self);
                PyroValue::I64(array.value(i))
            }
            DataType::UInt8 => {
                let array = as_primitive_array::<UInt8Type>(self);
                PyroValue::U8(array.value(i))
            }
            DataType::UInt16 => {
                let array = as_primitive_array::<UInt16Type>(self);
                PyroValue::U16(array.value(i))
            }
            DataType::UInt32 => {
                let array = as_primitive_array::<UInt32Type>(self);
                PyroValue::U32(array.value(i))
            }
            DataType::UInt64 => {
                let array = as_primitive_array::<UInt64Type>(self);
                PyroValue::U64(array.value(i))
            }
            DataType::Float16 => {
                let array = as_primitive_array::<Float16Type>(self);
                PyroValue::F16(array.value(i))
            }
            DataType::Float32 => {
                let array = as_primitive_array::<Float32Type>(self);
                PyroValue::F32(array.value(i))
            }
            DataType::Float64 => {
                let array = as_primitive_array::<Float64Type>(self);
                PyroValue::F64(array.value(i))
            }
            DataType::Date32 => {
                let array = as_primitive_array::<Date32Type>(self);
                PyroValue::I32(array.value(i))
            }
            DataType::Date64 => {
                let array = as_primitive_array::<Date64Type>(self);
                PyroValue::I64(array.value(i))
            }
            DataType::Boolean => {
                let array = as_boolean_array(self);
                PyroValue::Bool(array.value(i))
            }
            DataType::Binary => {
                let array = as_generic_binary_array::<i32>(self);
                PyroValue::PrimitiveList(PrimitiveValueList::U8(Cow::Borrowed(array.value(i))))
            }
            DataType::LargeBinary => {
                let array = as_generic_binary_array::<i64>(self);
                PyroValue::PrimitiveList(PrimitiveValueList::U8(Cow::Borrowed(array.value(i))))
            }
            DataType::BinaryView => {
                let array = as_generic_binary_array::<i32>(self);
                PyroValue::PrimitiveList(PrimitiveValueList::U8(Cow::Borrowed(array.value(i))))
            }
            DataType::FixedSizeBinary(_) => {
                let array = self
                    .as_any()
                    .downcast_ref::<FixedSizeBinaryArray>()
                    .expect("Just checked it has this type.");
                PyroValue::PrimitiveList(PrimitiveValueList::U8(Cow::Borrowed(array.value(i))))
            }
            DataType::Utf8 => {
                let array = as_string_array(self);
                PyroValue::Str(Cow::Borrowed(array.value(i)))
            }
            DataType::LargeUtf8 => {
                let array = as_largestring_array(self);
                PyroValue::Str(Cow::Borrowed(array.value(i)))
            }
            DataType::Time32(unit) => match unit {
                TimeUnit::Second => {
                    let array = as_primitive_array::<Time32SecondType>(self);
                    PyroValue::I32(array.value(i))
                }
                TimeUnit::Millisecond => {
                    let array = as_primitive_array::<Time32MillisecondType>(self);
                    PyroValue::I32(array.value(i))
                }
                _ => unreachable!(),
            },
            DataType::Time64(unit) => match unit {
                TimeUnit::Microsecond => {
                    let array = as_primitive_array::<Time64MicrosecondType>(self);
                    PyroValue::Timestamp(convert_u64_time(
                        array.value(i) as i128,
                        TimeUnit::Microsecond,
                    ))
                }
                TimeUnit::Nanosecond => {
                    let array = as_primitive_array::<Time64NanosecondType>(self);
                    PyroValue::Timestamp(convert_u64_time(
                        array.value(i) as i128,
                        TimeUnit::Nanosecond,
                    ))
                }
                _ => unreachable!(),
            },
            DataType::Timestamp(unit, tz) => {
                if let Some(timezone) = tz {
                    // Normalize checking for common UTC variants
                    let is_utc = matches!(
                        timezone.deref(),
                        "UTC" | "+00:00" | "Z" | "Etc/UTC" | "Greenwich"
                    );

                    if !is_utc {
                        return Err(ValueError::Unimplemented(
                            "ScalarValuable::scalar".to_string(),
                            format!(
                                "Non-UTC timezone '{}' is not supported. Only UTC or Naive timestamps can be converted to PyroValue::Timestamp.",
                                timezone
                            ),
                        ));
                    }
                }
                match unit {
                    TimeUnit::Second => {
                        let array = as_primitive_array::<TimestampSecondType>(self);
                        PyroValue::Timestamp(convert_u64_time(
                            array.value(i) as i128,
                            TimeUnit::Second,
                        ))
                    }
                    TimeUnit::Millisecond => {
                        let array = as_primitive_array::<TimestampMillisecondType>(self);
                        PyroValue::Timestamp(convert_u64_time(
                            array.value(i) as i128,
                            TimeUnit::Millisecond,
                        ))
                    }
                    TimeUnit::Microsecond => {
                        let array = as_primitive_array::<TimestampMicrosecondType>(self);
                        PyroValue::Timestamp(convert_u64_time(
                            array.value(i) as i128,
                            TimeUnit::Microsecond,
                        ))
                    }
                    TimeUnit::Nanosecond => {
                        let array = as_primitive_array::<TimestampNanosecondType>(self);
                        PyroValue::Timestamp(convert_u64_time(
                            array.value(i) as i128,
                            TimeUnit::Nanosecond,
                        ))
                    }
                }
            }
            DataType::Interval(_) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "IntervalMonthDayNano".to_string(),
                ));
            }
            DataType::Duration(unit) => match unit {
                TimeUnit::Second => {
                    let array = as_primitive_array::<DurationSecondType>(self);
                    PyroValue::I64(array.value(i))
                }
                TimeUnit::Millisecond => {
                    let array = as_primitive_array::<DurationMillisecondType>(self);
                    PyroValue::I64(array.value(i))
                }
                TimeUnit::Microsecond => {
                    let array = as_primitive_array::<DurationMicrosecondType>(self);
                    PyroValue::I64(array.value(i))
                }
                TimeUnit::Nanosecond => {
                    let array = as_primitive_array::<DurationNanosecondType>(self);
                    PyroValue::I64(array.value(i))
                }
            },
            DataType::Struct(_) => {
                let arrays = as_struct_array(self);
                let mut row = PyroRow::with_capacity(arrays.num_columns());

                for (field, array) in arrays.fields().iter().zip(arrays.columns()) {
                    let key = field.name().as_str();
                    // Safety:
                    // We are borrowing the key string from the schema which lives as long as the array ('a).
                    // PyroValueRef::Str expects &'a str.
                    // This unsafe cast connects the lifetime of the schema string to the lifetime of the return value.
                    let key_ref = unsafe { &*(key as *const str) };
                    row.insert_ref(key_ref, array.scalar(i)?);
                }
                PyroValue::Group(row)
            }
            DataType::Dictionary(key_type, _) => {
                let value = match key_type.deref() {
                    DataType::Int8 => {
                        let array = as_dictionary_array::<Int8Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::Int16 => {
                        let array = as_dictionary_array::<Int16Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::Int32 => {
                        let array = as_dictionary_array::<Int32Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::Int64 => {
                        let array = as_dictionary_array::<Int64Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::UInt8 => {
                        let array = as_dictionary_array::<UInt8Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::UInt16 => {
                        let array = as_dictionary_array::<UInt16Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::UInt32 => {
                        let array = as_dictionary_array::<UInt32Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    DataType::UInt64 => {
                        let array = as_dictionary_array::<UInt64Type>(self);
                        let index = array.keys().value(i).to_usize().unwrap();
                        array.values().scalar(index)
                    }
                    _ => unreachable!(),
                };

                value?
            }
            DataType::List(_) => {
                let array = self
                    .as_any()
                    .downcast_ref::<ListArray>()
                    .expect("Just checked it has this type.");
                if let Some(values) = unsafe { array.value(i).clone_as_list()? } {
                    PyroValue::PrimitiveList(values)
                } else {
                    let array_vals = array.value(i);
                    let mut values = Vec::with_capacity(array_vals.len());
                    for j in 0..array_vals.len() {
                        // Safety:
                        // We know the inner array values live as long as the outer array ('a).
                        // The iterator yields values with a shorter local lifetime, so we transmute
                        // to extend it to 'a.
                        let val = unsafe {
                            std::mem::transmute::<PyroValue<'_>, PyroValue<'a>>(
                                array_vals.scalar(j)?,
                            )
                        };
                        values.push(val);
                    }
                    PyroValue::List(values)
                }
            }
            DataType::LargeList(_) => {
                let array = self
                    .as_any()
                    .downcast_ref::<LargeListArray>()
                    .expect("Just checked it has this type.");
                if let Some(values) = unsafe { array.value(i).clone_as_list()? } {
                    PyroValue::PrimitiveList(values)
                } else {
                    let array_vals = array.value(i);
                    let mut values = Vec::with_capacity(array_vals.len());
                    for j in 0..array_vals.len() {
                        // Safety: See ListArray comment above.
                        let val = unsafe {
                            std::mem::transmute::<PyroValue<'_>, PyroValue<'a>>(
                                array_vals.scalar(j)?,
                            )
                        };
                        values.push(val);
                    }
                    PyroValue::List(values)
                }
            }
            DataType::FixedSizeList(_, _) => {
                let array = self
                    .as_any()
                    .downcast_ref::<FixedSizeListArray>()
                    .expect("Just checked it has this type.");
                if let Some(values) = unsafe { array.value(i).clone_as_list()? } {
                    PyroValue::PrimitiveList(values)
                } else {
                    let array_vals = array.value(i);
                    let mut values = Vec::with_capacity(array_vals.len());
                    for j in 0..array_vals.len() {
                        // Safety: See ListArray comment above.
                        let val = unsafe {
                            std::mem::transmute::<PyroValue<'_>, PyroValue<'a>>(
                                array_vals.scalar(j)?,
                            )
                        };
                        values.push(val);
                    }
                    PyroValue::List(values)
                }
            }
            DataType::Utf8View => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "Utf8View".to_string(),
                ));
            }
            DataType::ListView(_) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "ListView".to_string(),
                ));
            }
            DataType::LargeListView(_) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "LargeListView".to_string(),
                ));
            }
            DataType::Union(_, _) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "Union".to_string(),
                ));
            }
            DataType::Decimal128(_, _) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "Decimal128".to_string(),
                ));
            }
            DataType::Decimal256(_, _) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "Decimal256".to_string(),
                ));
            }
            DataType::RunEndEncoded(_, _) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "RunEndEncoded".to_string(),
                ));
            }
            DataType::Map(_, _) => {
                return Err(ValueError::Unimplemented(
                    "Array::scalar".to_string(),
                    "Map".to_string(),
                ));
            }
            DataType::Decimal32(_, _) => todo!(),
            DataType::Decimal64(_, _) => todo!(),
        };
        Ok(value)
    }
}

trait ListValuable {
    unsafe fn clone_as_list<'a>(&self) -> Result<Option<PrimitiveValueList<'a>>>;
}

impl<T: Array> ListValuable for T {
    /// Safety:
    /// You need to make sure the array you're borrowing this array from has the lifetime of 'a
    unsafe fn clone_as_list<'a>(&self) -> Result<Option<PrimitiveValueList<'a>>> {
        let values = match (self.data_type(), self.null_count()) {
            (DataType::Int8, 0) => {
                let array = as_primitive_array::<Int8Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::I8(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Int16, 0) => {
                let array = as_primitive_array::<Int16Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::I16(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Int32, 0) => {
                let array = as_primitive_array::<Int32Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::I32(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Int64, 0) => {
                let array = as_primitive_array::<Int64Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::I64(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::UInt8, 0) => {
                let array = as_primitive_array::<UInt8Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::U8(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::UInt16, 0) => {
                let array = as_primitive_array::<UInt16Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::U16(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::UInt32, 0) => {
                let array = as_primitive_array::<UInt32Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::U32(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::UInt64, 0) => {
                let array = as_primitive_array::<UInt64Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::U64(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Float16, 0) => {
                let array = as_primitive_array::<Float16Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::F16(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Float32, 0) => {
                let array = as_primitive_array::<Float32Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::F32(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Float64, 0) => {
                let array = as_primitive_array::<Float64Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::F64(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Date32, 0) => {
                let array = as_primitive_array::<Date32Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::I32(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            (DataType::Date64, 0) => {
                let array = as_primitive_array::<Date64Type>(self);
                let vals = &array.values()[..];
                let ptr = vals.as_ptr();
                Some(PrimitiveValueList::I64(Cow::Borrowed(unsafe {
                    std::slice::from_raw_parts(ptr, vals.len())
                })))
            }
            _ => None,
        };
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_bool_scalar() {
        let values = vec![Some(true), Some(false), None, Some(true), Some(false)];
        let array = BooleanArray::from(values);
        assert_eq!(array.scalar(0).unwrap(), PyroValue::Bool(true));
        assert_eq!(array.scalar(1).unwrap(), PyroValue::Bool(false));
        assert_eq!(array.scalar(2).unwrap(), PyroValue::Null);
    }

    #[test]
    fn test_primitive_int_scalar() {
        let values = vec![Some(1), Some(2), None, Some(3), Some(4)];
        let array = Int8Array::from(values);
        assert_eq!(array.scalar(0).unwrap(), PyroValue::I8(1),);
        assert_eq!(array.scalar(1).unwrap(), PyroValue::I8(2),);
        assert_eq!(array.scalar(2).unwrap(), PyroValue::Null);
    }

    #[test]
    fn test_string_scalar() {
        let values = vec![Some("1.0"), Some("2.0"), None, Some("3.0"), Some("4.0")];
        let array = StringArray::from(values);
        assert_eq!(array.scalar(0).unwrap(), PyroValue::from("1.0"));
        assert_eq!(array.scalar(1).unwrap(), PyroValue::from("2.0"));
        assert_eq!(array.scalar(2).unwrap(), PyroValue::Null);
    }

    #[test]
    fn test_large_string_scalar() {
        let values = vec![Some("1.0"), Some("2.0"), None, Some("3.0"), Some("4.0")];
        let array = LargeStringArray::from(values);
        assert_eq!(array.scalar(0).unwrap(), PyroValue::from("1.0"));
        assert_eq!(array.scalar(1).unwrap(), PyroValue::from("2.0"));
        assert_eq!(array.scalar(2).unwrap(), PyroValue::Null);
    }

    #[test]
    fn test_primitive_float_scalar() {
        let values = vec![Some(1.0), Some(2.0), None, Some(3.0), Some(4.0)];
        let array = Float32Array::from(values);
        assert_eq!(array.scalar(0).unwrap(), PyroValue::F32(1.0),);
        assert_eq!(array.scalar(1).unwrap(), PyroValue::F32(2.0),);
        assert_eq!(array.scalar(2).unwrap(), PyroValue::Null);
    }

    #[test]
    fn test_struct_scalar() {
        let values = vec![Some(true), Some(false), None, Some(true), Some(false)];
        let bool_array = Arc::new(BooleanArray::from(values));
        let values = vec![1.0, 2.0, 0.0, 3.0, 4.0];
        let float_array = Arc::new(Float64Array::from(values));
        let data = vec![
            Some(vec![Some(0), Some(1), Some(2)]),
            None,
            Some(vec![Some(3), None, Some(5)]),
            Some(vec![Some(6), Some(7)]),
            Some(vec![Some(8)]),
        ];
        let list_array = Arc::new(ListArray::from_iter_primitive::<Int32Type, _, _>(data));
        let schema = vec![
            (
                Arc::new(Field::new("bo", DataType::Boolean, true)),
                bool_array as ArrayRef,
            ),
            (
                Arc::new(Field::new("fl", DataType::Float64, false)),
                float_array as ArrayRef,
            ),
            (
                Arc::new(Field::new(
                    "list",
                    DataType::List(Arc::new(Field::new("item", DataType::Int32, true))),
                    true,
                )),
                list_array as ArrayRef,
            ),
        ];

        let struct_array = StructArray::from(schema);
        let zero_scalar = PyroValue::from([
            ("bo", PyroValue::Bool(true)),
            ("fl", PyroValue::F64(1.0)),
            ("list", PyroValue::from(&[0i32, 1, 2][..])),
        ]);

        let first_scalar = PyroValue::from([
            ("bo", PyroValue::Bool(false)),
            ("fl", PyroValue::F64(2.0)),
            ("list", PyroValue::Null),
        ]);

        let second_scalar = PyroValue::from([
            ("bo", PyroValue::Null),
            ("fl", PyroValue::F64(0.0)),
            ("list", PyroValue::from(vec![Some(3i32), None, Some(5)])),
        ]);

        assert_eq!(struct_array.scalar(0).unwrap(), zero_scalar);
        assert_eq!(struct_array.scalar(1).unwrap(), first_scalar);
        assert_eq!(struct_array.scalar(2).unwrap(), second_scalar);
    }

    #[test]
    fn test_dictionary_scalar() {
        let values = vec!["one", "one", "three", "one", "one"];
        let array: DictionaryArray<Int8Type> = values.into_iter().collect();

        assert_eq!(array.scalar(1).unwrap(), PyroValue::from("one"));
        assert_eq!(array.scalar(2).unwrap(), PyroValue::from("three"));
    }

    #[test]
    fn test_list_scalar() {
        let data = vec![
            Some(vec![Some(0), Some(1), Some(2)]),
            None,
            Some(vec![Some(3), None, Some(5)]),
            Some(vec![Some(6), Some(7)]),
            Some(vec![Some(8)]),
        ];
        let list_array = ListArray::from_iter_primitive::<Int32Type, _, _>(data);
        assert_eq!(
            list_array.scalar(2).unwrap(),
            PyroValue::from(vec![
                PyroValue::I32(3),
                PyroValue::Null,
                PyroValue::I32(5)
            ])
        );
    }

    #[test]
    fn test_dict_list_scalar() {
        let data = vec![
            Some(vec![Some(0), Some(1), Some(2)]),
            None,
            Some(vec![Some(3), None, Some(5)]),
            Some(vec![Some(6), Some(7)]),
            Some(vec![Some(8)]),
        ];
        let list = ListArray::from_iter_primitive::<Int32Type, _, _>(data);

        let keys = Int8Array::from(vec![0, 0, 0, 4, 3, 1, 2]);
        let array: Arc<DictionaryArray<Int8Type>> =
            Arc::new(DictionaryArray::try_new(keys, Arc::new(list)).unwrap());

        assert_eq!(
            array.scalar(2).unwrap(),
            PyroValue::from(&[0i32, 1, 2][..])
        );
    }
}
