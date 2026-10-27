//! Fuzz target: General fuzzing of the wire reader pipeline.
//!
//! Exercises the full `RowReader::from_parts` + `read_value`/`read_row` pipeline
//! with randomly constructed inode trees and arbitrary buffer contents.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyroformat::schema::{Inode, InodeMeta, KindMeta, WireKind, Word, INODE_SIZE};
use pyroformat::wire::value_readers::RowReader;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }

    // Low nibble: mode. High bit: structure word.
    let mode = (data[0] & 0x0F) % 6;
    let meta = InodeMeta::new(if data[0] & 0x80 != 0 { Word::U16 } else { Word::U32 });
    let mut seed = 1;

    match mode {
        0 => fuzz_single_inode_read(meta, data, &mut seed),
        1 => fuzz_inode_list_read(meta, data, &mut seed),
        2 => fuzz_roundtrip(meta, data, &mut seed),
        3 => fuzz_empty_and_edge_cases(meta, data),
        4 => fuzz_buffer_size_variants(meta, data, &mut seed),
        _ => fuzz_incomplete_decode(data),
    }
});

fn nth(data: &[u8], seed: &mut usize, max_val: usize) -> usize {
    if *seed >= data.len() { return 0; }
    let v = data[*seed] as usize;
    *seed += 1;
    v % max_val
}

fn single_kind(data: &[u8], seed: &mut usize) -> WireKind {
    let kinds = [
        WireKind::Bool, WireKind::I8, WireKind::I16, WireKind::I32, WireKind::I64,
        WireKind::U8, WireKind::U16, WireKind::U32, WireKind::U64,
        WireKind::F16, WireKind::F32, WireKind::F64, WireKind::Timestamp,
        WireKind::OptionBool, WireKind::OptionI8, WireKind::OptionI16,
        WireKind::OptionI32, WireKind::OptionI64, WireKind::OptionU8,
        WireKind::OptionU16, WireKind::OptionU32, WireKind::OptionU64,
        WireKind::OptionF16, WireKind::OptionF32, WireKind::OptionF64,
        WireKind::Str, WireKind::Bytes,
    ];
    let idx = nth(data, seed, kinds.len());
    kinds[idx]
}

fn make_single_inode(kind: WireKind, nesting: u8) -> Inode {
    Inode {
        kind,
        kind_meta: KindMeta::Primitive { nesting, data_offset: 0, topology_offset: 0, fixed_size: 0, nullable_bit_index: 0, group: false, group_nest: false },
    }
}

fn fuzz_single_inode_read(meta: InodeMeta, data: &[u8], seed: &mut usize) {
    if *seed >= data.len() { return; }
    let kind = single_kind(data, seed);
    let nesting = nth(data, seed, 3) as u8;
    let inode = make_single_inode(kind, nesting);
    let names: Vec<&str> = vec!["field_0"];

    if let Ok(reader) = RowReader::from_parts(meta, vec![inode], &names) {
        for size in [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024] {
            if size > data.len() { break; }
            let buf = vec![0u8; size];
            let _ = reader.read_value(&buf);
            let _ = reader.read_row(&buf);
        }
    }
}

fn fuzz_inode_list_read(meta: InodeMeta, data: &[u8], seed: &mut usize) {
    if *seed >= data.len() { return; }
    let num_fields = (nth(data, seed, 8) + 1) as usize;

    let mut inodes: Vec<Inode> = Vec::new();
    let kinds = [
        WireKind::I32, WireKind::U64, WireKind::F32, WireKind::Bool,
        WireKind::Str, WireKind::Bytes, WireKind::OptionI32, WireKind::Timestamp,
    ];

    for i in 0..num_fields {
        let kind = kinds[nth(data, seed, kinds.len())];
        let nesting = if i == 0 { 0 } else { nth(data, seed, 2) as u8 };

        inodes.push(make_single_inode(kind, nesting));
    }

    let field_names: Vec<String> = (0..inodes.len()).map(|i| format!("f{}", i)).collect();
    let names: Vec<&str> = field_names.iter().map(|s| s.as_str()).collect();

    if let Ok(reader) = RowReader::from_parts(meta, inodes, &names) {
        let buf_size = data.len().max(32).min(4096);
        let buf = vec![0u8; buf_size];
        let _ = reader.read_value(&buf);
        let _ = reader.read_row(&buf);
    }
}

fn fuzz_roundtrip(meta: InodeMeta, data: &[u8], seed: &mut usize) {
    if *seed >= data.len() { return; }
    let num_inodes = (nth(data, seed, 4) + 1) as usize;

    let kinds = [WireKind::I32, WireKind::U64, WireKind::F32, WireKind::Bool, WireKind::Str];
    let mut inodes: Vec<Inode> = Vec::new();
    for _ in 0..num_inodes {
        let kind = kinds[nth(data, seed, kinds.len())];
        inodes.push(make_single_inode(kind, 0));
    }

    let encoded: Vec<u8> = inodes.iter().flat_map(|i| i.encode().to_vec()).collect();
    let mut cursor = 0;
    let mut decoded: Vec<Inode> = Vec::new();
    while cursor + INODE_SIZE <= encoded.len() {
        if let Ok(inode) = Inode::decode(&encoded[cursor..cursor + INODE_SIZE]) {
            decoded.push(inode);
            cursor += INODE_SIZE;
        } else { break; }
    }

    let field_names: Vec<String> = (0..decoded.len()).map(|i| format!("r{}", i)).collect();
    let names: Vec<&str> = field_names.iter().map(|s| s.as_str()).collect();
    let _ = RowReader::from_parts(meta, decoded, &names);
}

fn fuzz_empty_and_edge_cases(meta: InodeMeta, _data: &[u8]) {
    let inode = make_single_inode(WireKind::I32, 0);
    let names: Vec<&str> = vec!["f0"];

    if let Ok(reader) = RowReader::from_parts(meta, vec![inode], &names) {
        let buf: [u8; 0] = [];
        let _ = reader.read_value(&buf);
        let _ = reader.read_row(&buf);

        let buf = vec![0u8; 64];
        let _ = reader.read_value(&buf);
        let _ = reader.read_row(&buf);

        let buf = vec![0xFFu8; 64];
        let _ = reader.read_value(&buf);
        let _ = reader.read_row(&buf);

        let buf: Vec<u8> = (0..256).map(|i| if i % 2 == 0 { 0x00 } else { 0xFF }).collect();
        let _ = reader.read_value(&buf);
        let _ = reader.read_row(&buf);
    }

    // Invalid inodes
    let unknown = Inode {
        kind: WireKind::Unknown,
        kind_meta: KindMeta::Primitive { nesting: 0, data_offset: 0, topology_offset: 0, fixed_size: 0, nullable_bit_index: 0, group: false, group_nest: false },
    };
    let _ = RowReader::from_parts(meta, vec![unknown], &[]);

    // Empty inodes
    let _ = RowReader::from_parts(meta, Vec::new(), &["field0"]);
}

fn fuzz_buffer_size_variants(meta: InodeMeta, data: &[u8], seed: &mut usize) {
    if *seed >= data.len() { return; }
    let kind = match nth(data, seed, 8) {
        0 => WireKind::I8, 1 => WireKind::I16, 2 => WireKind::I32, 3 => WireKind::I64,
        4 => WireKind::U8, 5 => WireKind::U16, 6 => WireKind::F32, _ => WireKind::F64,
    };
    let inode = make_single_inode(kind, 0);
    let names: Vec<&str> = vec!["x"];

    if let Ok(reader) = RowReader::from_parts(meta, vec![inode], &names) {
        let sizes: Vec<usize> = (0..20).map(|p| 1 << p).collect();
        for size in sizes {
            if size > 1_048_576 { break; }
            let buf = vec![0u8; size];
            let _ = reader.read_value(&buf);
        }
    }
}

fn fuzz_incomplete_decode(data: &[u8]) {
    let max_offset = data.len().min(INODE_SIZE + 10);
    for offset in 0..max_offset {
        let remaining = data.len() - offset;
        let slice_len = remaining.min(INODE_SIZE);
        if slice_len > 0 {
            let _ = Inode::decode(&data[offset..offset + slice_len]);
        }
    }
}
