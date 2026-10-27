use pyro_schema::{
    Inode, SchemaParts, human::PyroSchema as HumanSchema, inode_data_len, schema_hash,
};

use crate::{
    PyroValue,
    buf::{PyroSink, sink::LocalMemSink},
    wire::{value_readers::RowReader, value_writers::ValueWriter},
};

/// `#[track_caller]` so every `assert_eq!`/`.unwrap()` panic below reports
/// the call site in the actual test (`src/wire/tests/*.rs`) instead of a
/// line number in this shared helper.
#[track_caller]
pub fn run_list_wrapped_roundtrip_u32(
    human_schema: HumanSchema,
    val: PyroValue,
    expected_buffer: &[u8],
    expected_inodes: &[Inode],
) {
    let mut schema = human_schema.to_machine();
    schema.set_word(pyro_schema::Word::U32);
    let vw = ValueWriter::new(&schema).unwrap();
    let schema = SchemaParts::from_schema(&schema).unwrap();
    let schema_encoded = schema.encode(true);
    let schema_decoded = SchemaParts::decode(&schema_encoded).unwrap();
    assert_eq!(schema_decoded, schema);
    let human_decoded: HumanSchema = schema_decoded.to_schema().unwrap().into();
    assert_eq!(human_decoded, human_schema);

    assert_eq!(schema.inodes, expected_inodes);
    println!("Inodes: {:#?}", schema.inodes);
    println!("Value: {:?}", val);
    println!("Expected Buffer bytes: {:?}", &expected_buffer[..]);
    let vw_b = vw.ingest(&val).unwrap();
    tracing::info!("Encoded length: {:?}", vw_b.variable_len());
    assert_eq!(vw_b.fixed_len(), inode_data_len::<u32>(&schema.inodes));
    let mut sink = LocalMemSink::new(vw_b.fixed_len(), schema_hash(&schema.meta, &schema.inodes));
    sink.reallocate_sync(vw_b.variable_len()).unwrap();
    vw_b.write(&mut sink).unwrap();
    println!("Actual Buffer bytes: {:?}", &sink.payload());
    assert_eq!(sink.payload(), expected_buffer);
    let vr = RowReader::from_parts(schema.meta, expected_inodes.to_vec(), &schema.field).unwrap();
    let restored = vr.read_value(sink.payload()).unwrap();
    println!("Restored: {:?}", restored);
    assert_eq!(val, restored);
}

/// `#[track_caller]` so every `assert_eq!`/`.unwrap()` panic below reports
/// the call site in the actual test (`src/wire/tests/*.rs`) instead of a
/// line number in this shared helper.
#[track_caller]
pub fn run_list_wrapped_roundtrip_u16(
    human_schema: HumanSchema,
    val: PyroValue,
    expected_buffer: &[u8],
    expected_inodes: &[Inode],
) {
    let mut schema = human_schema.to_machine();
    schema.set_word(pyro_schema::Word::U16);
    let vw = ValueWriter::new(&schema).unwrap();
    let schema = SchemaParts::from_schema(&schema).unwrap();
    let schema_encoded = schema.encode(true);
    let schema_decoded = SchemaParts::decode(&schema_encoded).unwrap();
    assert_eq!(schema_decoded, schema);
    let human_decoded: HumanSchema = schema_decoded.to_schema().unwrap().into();
    assert_eq!(human_decoded, human_schema);

    assert_eq!(schema.inodes, expected_inodes);
    println!("Inodes: {:#?}", schema.inodes);
    println!("Value: {:?}", val);
    println!("Expected Buffer bytes: {:?}", &expected_buffer[..]);
    let vw_b = vw.ingest(&val).unwrap();
    tracing::info!("Encoded length: {:?}", vw_b.variable_len());
    assert_eq!(vw_b.fixed_len(), inode_data_len::<u16>(&schema.inodes));
    let mut sink = LocalMemSink::new(vw_b.fixed_len(), schema_hash(&schema.meta, &schema.inodes));
    sink.reallocate_sync(vw_b.variable_len()).unwrap();
    vw_b.write(&mut sink).unwrap();
    println!("Actual Buffer bytes: {:?}", &sink.payload());
    assert_eq!(sink.payload(), expected_buffer);
    let vr = RowReader::from_parts(schema.meta, expected_inodes.to_vec(), &schema.field).unwrap();
    let restored = vr.read_value(&sink.payload()).unwrap();
    println!("Restored: {:?}", restored);
    assert_eq!(val, restored);
}
