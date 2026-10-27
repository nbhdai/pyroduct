//! Fuzz target: Fuzz basic inode setups — flat, non-nested field types.
//!
//! This target constructs `Inode` structures programmatically covering
//! all basic wire kinds (primitives, strings, bytes, timestamps) at
//! nesting depth 0, then exercises the full read pipeline.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyroformat::schema::{Inode, InodeMeta, KindMeta, WireKind, Word};
use pyroformat::wire::value_readers::RowReader;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }

    // Low 5 bits: field count. High bit: structure word.
    let num_fields = (data[0] % 32) as usize;
    let meta = InodeMeta::new(if data[0] & 0x80 != 0 { Word::U16 } else { Word::U32 });
    let mut inodes: Vec<Inode> = Vec::with_capacity(num_fields);
    let mut seed = 1;

    let kinds = [
        WireKind::Bool, WireKind::I8, WireKind::I16, WireKind::I32, WireKind::I64,
        WireKind::U8, WireKind::U16, WireKind::U32, WireKind::U64,
        WireKind::F16, WireKind::F32, WireKind::F64, WireKind::Timestamp,
        WireKind::OptionBool, WireKind::OptionI8, WireKind::OptionI32,
        WireKind::OptionI64, WireKind::OptionU8, WireKind::OptionU16,
        WireKind::OptionU32, WireKind::OptionU64, WireKind::OptionF32,
        WireKind::OptionF64, WireKind::Str, WireKind::Bytes,
    ];

    for _ in 0..num_fields {
        if seed >= data.len() { break; }
        let kind = kinds[data[seed] as usize % kinds.len()];
        seed += 1;

        inodes.push(Inode {
            kind,
            kind_meta: KindMeta::Primitive {
                nesting: 0,
                data_offset: 0,
                topology_offset: 0,
                fixed_size: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        });
    }

    let field_names: Vec<String> = (0..inodes.len()).map(|i| format!("field_{}", i)).collect();
    let names: Vec<&str> = field_names.iter().map(|s| s.as_str()).collect();

    match RowReader::from_parts(meta, inodes.clone(), &names) {
        Ok(reader) => {
            let buf_size = data.len().max(64).min(65536);
            let buf = vec![0u8; buf_size];
            let _ = reader.read_value(&buf);
            let _ = reader.read_row(&buf);
        }
        Err(_) => {}
    }

    // Also test with a single root inode
    if !inodes.is_empty() {
        let _ = RowReader::from_parts(meta, vec![inodes[0]], &[]);
    }
});
