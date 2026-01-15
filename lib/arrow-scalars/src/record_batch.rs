use std::sync::Arc;

use arrow::array::*;
use arrow::buffer::{NullBuffer, OffsetBuffer, ScalarBuffer};
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use arrow_schema::SchemaRef;

use crate::{
    ArrowRow, ArrowRowOwned, ArrowScalarError, ArrowValue, PrimitiveValueList, ScalarRepairError,
    ScalarValuable,
};

pub trait Rowable {
    fn row(&self, index: usize) -> Result<ArrowRow<'_>, ArrowScalarError>;
    fn rows(&self) -> impl Iterator<Item = Result<ArrowRow<'_>, ArrowScalarError>>;
    fn project_row(
        &self,
        schema: &Schema,
        index: usize,
    ) -> std::result::Result<Option<ArrowRow<'_>>, ScalarRepairError>;
}

impl<T: Rowable> Rowable for &T {
    fn row(&self, index: usize) -> Result<ArrowRow<'_>, ArrowScalarError> {
        (*self).row(index)
    }

    fn rows(&self) -> impl Iterator<Item = Result<ArrowRow<'_>, ArrowScalarError>> {
        (*self).rows()
    }

    fn project_row(
        &self,
        schema: &Schema,
        index: usize,
    ) -> std::result::Result<Option<ArrowRow<'_>>, ScalarRepairError> {
        (*self).project_row(schema, index)
    }
}

impl<T: ?Sized + Rowable> Rowable for Box<T> {
    fn row(&self, index: usize) -> Result<ArrowRow<'_>, ArrowScalarError> {
        (**self).row(index)
    }

    fn rows(&self) -> impl Iterator<Item = Result<ArrowRow<'_>, ArrowScalarError>> {
        (**self).rows()
    }

    fn project_row(
        &self,
        schema: &Schema,
        index: usize,
    ) -> std::result::Result<Option<ArrowRow<'_>>, ScalarRepairError> {
        (**self).project_row(schema, index)
    }
}

impl<T: ?Sized + Rowable> Rowable for Arc<T> {
    fn row(&self, index: usize) -> Result<ArrowRow<'_>, ArrowScalarError> {
        (**self).row(index)
    }

    fn rows(&self) -> impl Iterator<Item = Result<ArrowRow<'_>, ArrowScalarError>> {
        (**self).rows()
    }

    fn project_row(
        &self,
        schema: &Schema,
        index: usize,
    ) -> std::result::Result<Option<ArrowRow<'_>>, ScalarRepairError> {
        (**self).project_row(schema, index)
    }
}

impl Rowable for RecordBatch {
    fn row<'a>(&'a self, index: usize) -> Result<ArrowRow<'a>, ArrowScalarError> {
        self.schema()
            .fields()
            .iter()
            .zip(self.columns())
            .map(|(field, array)| {
                let key = field.name().as_str();
                // Safe because we're borrowing from our schema which has the same lifetime as the record batch
                Ok((unsafe { &*(key as *const str) }, array.scalar(index)?))
            })
            .collect()
    }

    fn rows<'a>(&'a self) -> impl Iterator<Item = Result<ArrowRow<'a>, ArrowScalarError>> {
        (0..self.num_rows()).map(|i| self.row(i))
    }

    fn project_row(
        &self,
        schema: &Schema,
        index: usize,
    ) -> std::result::Result<Option<ArrowRow<'_>>, ScalarRepairError> {
        let my_schema = self.schema();
        let mut val = ArrowRow::with_capacity(schema.fields().len());
        for (key, array) in schema.fields().iter().filter_map(|field| {
            self.column_by_name(field.name()).map(|a| {
                let (_, my_field) = my_schema
                    .column_with_name(field.name())
                    .expect("Can't get here without this being non-null");
                let key = my_field.name().as_str();
                (unsafe { &*(key as *const str) }, a)
            })
        }) {
            let row = array.scalar(index).ok();
            if let Some(row) = row {
                val.insert(key.to_string(), ArrowValue::from(row));
            } else {
                return Ok(None);
            }
        }
        Ok(Some(val))
    }
}

impl Rowable for StructArray {
    fn row<'a>(&'a self, index: usize) -> Result<ArrowRow<'a>, ArrowScalarError> {
        self.fields()
            .iter()
            .zip(self.columns())
            .map(|(field, array)| {
                let key = field.name().as_str();
                Ok((unsafe { &*(key as *const str) }, array.scalar(index)?))
            })
            .collect()
    }

    fn rows<'a>(&'a self) -> impl Iterator<Item = Result<ArrowRow<'a>, ArrowScalarError>> {
        (0..self.len()).map(|i| self.row(i))
    }

    fn project_row(
        &self,
        schema: &Schema,
        index: usize,
    ) -> std::result::Result<Option<ArrowRow<'_>>, ScalarRepairError> {
        let mut val = ArrowRow::with_capacity(schema.fields().len());
        for (key, array) in schema
            .fields()
            .iter()
            .filter_map(|field| self.column_by_name(field.name()).map(|a| (field.name(), a)))
        {
            let row = array.scalar(index).ok();
            if let Some(row) = row {
                val.insert(key.to_string(), ArrowValue::from(row));
            } else {
                return Ok(None);
            }
        }
        Ok(Some(val))
    }
}

/// Macro to build primitive arrays (Int32Array, etc.)
macro_rules! build_primitive {
    ($values:expr, $builder_type:ty, $variant:ident) => {{
        let mut builder = <$builder_type>::with_capacity($values.len());
        for v in $values {
            match v {
                ArrowValue::$variant(val) => builder.append_value(*val),
                ArrowValue::Null => builder.append_null(),
                _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
            }
        }
        Ok(Arc::new(builder.finish()))
    }};
}

/// Macro to build Lists of Primitives with optimizations for `PrimitiveValueList`
/// and coercion for `Vec<ArrowValue>`.
macro_rules! build_list_primitive {
    ($values:expr, $inner_builder_ty:ty, $scalar_variant:ident, $list_variant:ident) => {{
        let mut builder = ListBuilder::new(<$inner_builder_ty>::with_capacity($values.len() * 5));

        for v in $values {
            match v {
                ArrowValue::Null => builder.append_null(),

                // Optimization: Zero-copy append slice
                ArrowValue::PrimitiveList(PrimitiveValueList::$list_variant(cow)) => {
                    builder.values().append_slice(cow.as_ref());
                    builder.append(true);
                }

                // Coercion: Unpack Vec<ArrowValue> -> Primitive Builder
                ArrowValue::List(list) => {
                    for item in list {
                        match item {
                            ArrowValue::$scalar_variant(val) => builder.values().append_value(*val),
                            ArrowValue::Null => builder.values().append_null(),
                            _ => {
                                return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned()))
                            }
                        }
                    }
                    builder.append(true);
                }

                _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
            }
        }
        Ok(Arc::new(builder.finish()))
    }};
}

/// A buffer for accumulating rows before flushing them to an immutable Arrow RecordBatch.
pub struct PreBatch {
    schema: SchemaRef,
    rows: Vec<ArrowRowOwned>,
}

impl PreBatch {
    pub fn new(schema: SchemaRef) -> Self {
        Self {
            schema,
            rows: Vec::new(),
        }
    }

    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn push<'a>(&mut self, row: ArrowRow<'a>) -> Result<(), ScalarRepairError> {
        let row = row.project_repair(self.schema.fields())?;
        self.rows.push(row.into_owned());
        Ok(())
    }

    pub fn push_unchecked<'a>(&mut self, row: ArrowRow<'a>) {
        // In a production system, we might validate the row against the schema here.
        // For performance, we assume the caller (or the repair logic) has ensured consistency.
        self.rows.push(row.into_owned());
    }

    pub fn flush(&mut self) -> Result<Option<RecordBatch>, ArrowScalarError> {
        if self.rows.is_empty() {
            return Ok(None);
        }

        let columns = self
            .schema
            .fields()
            .iter()
            .map(|field| {
                let col_values: Vec<ArrowValue> = self
                    .rows
                    .iter()
                    .map(|row| row.get(field.name()).cloned().unwrap_or(ArrowValue::Null))
                    .collect();
                build_array(field.data_type(), &col_values)
            })
            .collect::<Result<Vec<ArrayRef>, ArrowScalarError>>()?;

        let batch = RecordBatch::try_new(self.schema.clone(), columns)?;

        self.rows.clear();
        Ok(Some(batch))
    }
}

/// Recursively builds an Arrow Array from a slice of ArrowValues based on the target DataType.
fn build_array(data_type: &DataType, values: &[ArrowValue]) -> Result<ArrayRef, ArrowScalarError> {
    match data_type {
        DataType::Null => {
            let array = NullArray::new(values.len());
            Ok(Arc::new(array))
        }
        DataType::Boolean => {
            let mut builder = BooleanBuilder::with_capacity(values.len());
            for v in values {
                match v {
                    ArrowValue::Bool(b) => builder.append_value(*b),
                    ArrowValue::Null => builder.append_null(),
                    _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Int8 => build_primitive!(values, Int8Builder, I8),
        DataType::Int16 => build_primitive!(values, Int16Builder, I16),
        DataType::Int32 => build_primitive!(values, Int32Builder, I32),
        DataType::Int64 => build_primitive!(values, Int64Builder, I64),
        DataType::UInt8 => build_primitive!(values, UInt8Builder, U8),
        DataType::UInt16 => build_primitive!(values, UInt16Builder, U16),
        DataType::UInt32 => build_primitive!(values, UInt32Builder, U32),
        DataType::UInt64 => build_primitive!(values, UInt64Builder, U64),
        DataType::Float16 => build_primitive!(values, Float16Builder, F16),
        DataType::Float32 => build_primitive!(values, Float32Builder, F32),
        DataType::Float64 => build_primitive!(values, Float64Builder, F64),

        DataType::Utf8 => {
            let mut builder = StringBuilder::with_capacity(values.len(), values.len() * 5);
            for v in values {
                match v {
                    ArrowValue::Str(s) => builder.append_value(s.as_ref()),
                    ArrowValue::Null => builder.append_null(),
                    _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
                }
            }
            Ok(Arc::new(builder.finish()))
        }

        DataType::LargeUtf8 => {
            let mut builder = LargeStringBuilder::with_capacity(values.len(), values.len() * 5);
            for v in values {
                match v {
                    ArrowValue::Str(s) => builder.append_value(s.as_ref()),
                    ArrowValue::Null => builder.append_null(),
                    _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
                }
            }
            Ok(Arc::new(builder.finish()))
        }

        DataType::Struct(fields) => {
            // Transpose Row-based ArrowValues into Column-based arrays recursively
            let mut arrays = Vec::with_capacity(fields.len());

            for field in fields {
                let field_name = field.name();
                let mut field_values = Vec::with_capacity(values.len());

                for v in values {
                    match v {
                        ArrowValue::Group(row) => {
                            let val = row.get(field_name).cloned().unwrap_or(ArrowValue::Null);
                            field_values.push(val);
                        }
                        ArrowValue::Null => {
                            // If the struct is null, its children are null
                            field_values.push(ArrowValue::Null);
                        }
                        _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
                    }
                }

                let array = build_array(field.data_type(), &field_values)?;
                arrays.push(array);
            }

            // Calculate validity bitmap for the struct itself
            let nulls: Option<NullBuffer> = if values.iter().any(|v| v.is_null()) {
                Some(values.iter().map(|v| !v.is_null()).collect())
            } else {
                None
            };

            Ok(Arc::new(StructArray::new(fields.clone(), arrays, nulls)))
        }

        DataType::List(field) => {
            // Specific optimization logic for primitive lists requested by user
            match field.data_type() {
                DataType::Int8 => build_list_primitive!(values, Int8Builder, I8, I8),
                DataType::Int16 => build_list_primitive!(values, Int16Builder, I16, I16),
                DataType::Int32 => build_list_primitive!(values, Int32Builder, I32, I32),
                DataType::Int64 => build_list_primitive!(values, Int64Builder, I64, I64),
                DataType::UInt8 => build_list_primitive!(values, UInt8Builder, U8, U8),
                DataType::UInt16 => build_list_primitive!(values, UInt16Builder, U16, U16),
                DataType::UInt32 => build_list_primitive!(values, UInt32Builder, U32, U32),
                DataType::UInt64 => build_list_primitive!(values, UInt64Builder, U64, U64),
                DataType::Float32 => build_list_primitive!(values, Float32Builder, F32, F32),
                DataType::Float64 => build_list_primitive!(values, Float64Builder, F64, F64),
                _ => build_generic_list::<i32>(field, values),
            }
        }

        DataType::LargeList(field) => {
            // We can reuse the primitive macros if we adjusted them for OffsetSize,
            // but for simplicity and robustness we use the generic builder for LargeList for now
            // unless we want to duplicate macros for LargeListBuilder.
            // Given the "optimization" request was likely for standard List<Primitive>,
            // we can route all LargeLists to the generic path or add LargeList support to macros.
            // To keep it clean, we route to generic path which is correct but maybe less "zero-copy" optimized than the macro.
            build_generic_list::<i64>(field, values)
        }

        _ => Err(ArrowScalarError::Unimplemented(
            "build_array".to_string(),
            format!("{:?}", data_type),
        )),
    }
}

/// Helper to build a Generic List (nested, strings, structs, etc.) by flattening the values
/// and constructing the ListArray directly. This avoids the recursion limits of ArrayBuilder.
fn build_generic_list<O: OffsetSizeTrait>(
    field: &FieldRef,
    values: &[ArrowValue],
) -> Result<ArrayRef, ArrowScalarError> {
    let mut flat_values = Vec::new();
    let mut offsets = Vec::with_capacity(values.len() + 1);
    let mut valid_mask = Vec::with_capacity(values.len());

    let mut current_offset = O::zero();
    offsets.push(current_offset);

    for v in values {
        match v {
            ArrowValue::Null => {
                valid_mask.push(false);
                offsets.push(current_offset);
            }
            ArrowValue::List(items) => {
                valid_mask.push(true);
                current_offset += O::from_usize(items.len()).unwrap();
                offsets.push(current_offset);
                flat_values.extend(items.iter().cloned());
            }
            // Handle Coercion from PrimitiveList if it reaches here (e.g. mixed types or LargeList<Primitive>)
            ArrowValue::PrimitiveList(pl) => {
                // We need to convert PrimitiveList back to ArrowValues to put in flat_values
                // This is slower than the optimized macro path, but correct.
                // We can't easily zero-copy here because flat_values is Vec<ArrowValue>.
                valid_mask.push(true);

                // Helper to expand PrimitiveValueList
                let count = match pl {
                    PrimitiveValueList::Bool(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::Bool(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::I8(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::I8(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::I16(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::I16(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::I32(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::I32(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::I64(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::I64(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::U8(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::U8(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::U16(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::U16(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::U32(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::U32(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::U64(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::U64(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::F32(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::F32(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::F64(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::F64(x));
                        }
                        c.len()
                    }
                    PrimitiveValueList::F16(c) => {
                        for &x in c.as_ref() {
                            flat_values.push(ArrowValue::F16(x));
                        }
                        c.len()
                    }
                };
                current_offset += O::from_usize(count).unwrap();
                offsets.push(current_offset);
            }
            _ => return Err(ArrowScalarError::InvalidScalar(v.clone().into_owned())),
        }
    }

    // Recursively build the child array
    let child_array = build_array(field.data_type(), &flat_values)?;

    let offsets_buffer = OffsetBuffer::new(ScalarBuffer::from(offsets));
    let null_buffer = NullBuffer::from(valid_mask);

    Ok(Arc::new(GenericListArray::<O>::new(
        field.clone(),
        offsets_buffer,
        child_array,
        Some(null_buffer),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrowRow, ArrowValue};
    use arrow::datatypes::Field;

    #[test]
    fn test_prebatch_primitive() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("i32", DataType::Int32, true),
            Field::new("str", DataType::Utf8, true),
        ]));

        let mut batch = PreBatch::new(schema.clone());

        batch
            .push(
                ArrowRow::from([
                    ("i32", ArrowValue::I32(10)),
                    ("str", ArrowValue::from("ten")),
                ])
                .into_owned(),
            )
            .unwrap();

        batch
            .push(
                ArrowRow::from([
                    ("i32", ArrowValue::Null),
                    ("str", ArrowValue::from("null_int")),
                ])
                .into_owned(),
            )
            .unwrap();

        let record_batch = batch.flush().unwrap().unwrap();

        assert_eq!(record_batch.num_rows(), 2);

        let int_col = record_batch.column(0).as_primitive::<Int32Type>();
        assert_eq!(int_col.value(0), 10);
        assert!(int_col.is_null(1));

        let str_col = record_batch.column(1).as_string::<i32>();
        assert_eq!(str_col.value(0), "ten");
        assert_eq!(str_col.value(1), "null_int");
    }

    #[test]
    fn test_prebatch_struct() {
        let inner_fields = vec![Arc::new(Field::new("val", DataType::Int32, false))];
        let schema = Arc::new(Schema::new(vec![Field::new(
            "wrap",
            DataType::Struct(inner_fields.into()),
            true,
        )]));

        let mut batch = PreBatch::new(schema);

        batch
            .push(
                ArrowRow::from([(
                    "wrap",
                    ArrowValue::from(ArrowRow::from([("val", ArrowValue::I32(99))])),
                )])
                .into_owned(),
            )
            .unwrap();

        let rb = batch.flush().unwrap().unwrap();
        let struct_col = rb.column(0).as_struct();
        let inner_col = struct_col.column(0).as_primitive::<Int32Type>();
        assert_eq!(inner_col.value(0), 99);
    }

    #[test]
    fn test_list_optimization_and_coercion() {
        // Schema: List<Int32>
        let schema = Arc::new(Schema::new(vec![Field::new(
            "list",
            DataType::List(Arc::new(Field::new("item", DataType::Int32, true))),
            true,
        )]));

        let mut batch = PreBatch::new(schema);

        // 1. Push Optimized PrimitiveList (Zero Copy path)
        batch
            .push(ArrowRow::from([("list", ArrowValue::from(&[1i32, 2, 3][..]))]).into_owned())
            .unwrap();

        // 2. Push Generic List (Coercion path: Vec<ArrowValue> -> Builder)
        batch
            .push(
                ArrowRow::from([(
                    "list",
                    ArrowValue::List(vec![ArrowValue::I32(4), ArrowValue::I32(5)]),
                )])
                .into_owned(),
            )
            .unwrap();

        // 3. Push Null List
        batch
            .push(ArrowRow::from([("list", ArrowValue::Null)]).into_owned())
            .unwrap();

        let rb = batch.flush().unwrap().unwrap();
        let list_arr = rb.column(0).as_list::<i32>();

        // Check Row 1 (Optimized)
        assert_eq!(
            list_arr.value(0).as_primitive::<Int32Type>().values(),
            &[1, 2, 3]
        );

        // Check Row 2 (Coerced)
        assert_eq!(
            list_arr.value(1).as_primitive::<Int32Type>().values(),
            &[4, 5]
        );

        // Check Row 3 (Null)
        assert!(list_arr.is_null(2));
    }

    #[test]
    fn test_generic_list_of_structs() {
        // Schema: List<Struct<Int>>
        let struct_field =
            DataType::Struct(vec![Arc::new(Field::new("a", DataType::Int32, false))].into());
        let schema = Arc::new(Schema::new(vec![Field::new(
            "complex_list",
            DataType::List(Arc::new(Field::new("item", struct_field, true))),
            true,
        )]));

        let mut batch = PreBatch::new(schema);

        // Row 1: List of 2 Structs
        batch
            .push(
                ArrowRow::from([(
                    "complex_list",
                    ArrowValue::List(vec![
                        ArrowValue::from(ArrowRow::from([("a", ArrowValue::I32(10))])),
                        ArrowValue::from(ArrowRow::from([("a", ArrowValue::I32(20))])),
                    ]),
                )])
                .into_owned(),
            )
            .unwrap();

        let rb = batch.flush().unwrap().unwrap();
        let list_arr = rb.column(0).as_list::<i32>();

        assert_eq!(list_arr.len(), 1);
        let row0 = list_arr.value(0);
        let structs = row0.as_struct();
        let ints = structs.column(0).as_primitive::<Int32Type>();

        assert_eq!(ints.values(), &[10, 20]);
    }
}
