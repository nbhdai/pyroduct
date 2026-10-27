//! Fuzz target: Decode raw bytes into `Inode` structures and exercise
//! the Inode decode/encode round-trip, as well as the wire reader
//! pipeline on arbitrary (possibly malformed) inode data.
//!
//! This target feeds raw byte slices directly into `Inode::decode`
//! and `KindMeta::decode` to catch panics and logic errors in inode
//! parsing. It then attempts to build a `Reader` from whatever
//! inodes it managed to produce.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyroformat::schema::{Inode, InodeMeta, Word, INODE_SIZE};
use pyroformat::wire::value_readers::RowReader;

fuzz_target!(|data: &[u8]| {
    // Structure word for every reader built below: first byte's low bit.
    let meta = InodeMeta::new(match data.first() {
        Some(b) if b & 1 == 1 => Word::U16,
        _ => Word::U32,
    });

    // --- 0. InodeMeta (the table's first slot) decode/encode round-trip ---
    if data.len() >= INODE_SIZE {
        if let Ok(decoded_meta) = InodeMeta::decode(&data[..INODE_SIZE]) {
            let re = InodeMeta::decode(&decoded_meta.encode()).expect("re-decode of encoded InodeMeta");
            assert_eq!(decoded_meta, re);
        }
    }
    for word in [Word::U16, Word::U32] {
        let m = InodeMeta::new(word);
        assert_eq!(InodeMeta::decode(&m.encode()).ok(), Some(m));
    }

    // --- 1. Decode individual inodes from arbitrary byte slices ---

    let mut offset = 0;
    while offset + INODE_SIZE <= data.len() {
        if let Ok(inode) = Inode::decode(&data[offset..offset + INODE_SIZE]) {
            // Round-trip: encode back and re-decode
            let encoded = inode.encode();
            if let Ok(decoded) = Inode::decode(&encoded) {
                assert_eq!(inode.kind, decoded.kind);
                assert_eq!(inode.kind_meta, decoded.kind_meta);
            }

            // Attempt to build a Reader from this single inode
            let _ = try_build_reader(meta, &[inode], &[]);
        }
        offset += 1;
    }

    // --- 2. Bulk inode list decoding ---

    let mut cursor = 0;
    let mut inodes: Vec<Inode> = Vec::new();

    let count: usize = if cursor + 4 <= data.len() {
        let count_bytes = [data[cursor], data[cursor + 1], data[cursor + 2], data[cursor + 3]];
        cursor += 4;
        u32::from_le_bytes(count_bytes) as usize % 100
    } else {
        0
    };

    for _ in 0..count {
        if cursor + INODE_SIZE <= data.len() {
            if let Ok(inode) = Inode::decode(&data[cursor..cursor + INODE_SIZE]) {
                inodes.push(inode);
                cursor += INODE_SIZE;
            } else {
                cursor += 1;
            }
        } else {
            break;
        }
    }

    let field_names: Vec<String> = (0..inodes.len()).map(|i| format!("field_{}", i)).collect();
    let names: Vec<&str> = field_names.iter().map(|s| s.as_str()).collect();
    let _ = try_build_reader(meta, &inodes, &names);
});

pub fn try_build_reader(meta: InodeMeta, inodes: &[Inode], field_names: &[&str]) -> Option<()> {
    if let Ok(reader) = RowReader::from_parts(meta, inodes.to_vec(), field_names) {
        let buf = [0u8; 1024];
        let _ = reader.read_value(&buf);
        let _ = reader.read_row(&buf);
    }
    Some(())
}
