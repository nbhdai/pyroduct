//! Fuzz target: Fuzz nested lists and group structures.
//!
//! This target constructs complex `Inode` hierarchies including nested
//! primitive lists, groups with children, lists of groups, and deeply
//! nested structures.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyroformat::schema::{Inode, InodeMeta, KindMeta, WireKind, Word};
use pyroformat::wire::value_readers::RowReader;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }

    // Low nibble: builder mode. High bit: structure word.
    let mode = (data[0] & 0x0F) % 12;
    let meta = InodeMeta::new(if data[0] & 0x80 != 0 { Word::U16 } else { Word::U32 });
    let mut seed = 1;
    let inodes = build_inodes(mode, data, &mut seed);

    if !inodes.is_empty() {
        let field_names: Vec<String> = (0..inodes.len()).map(|i| format!("f{}", i)).collect();
        let names: Vec<&str> = field_names.iter().map(|s| s.as_str()).collect();
        if let Ok(reader) = RowReader::from_parts(meta, inodes, &names) {
            let buf_size = data.len().max(64).min(65536);
            let buf = vec![0u8; buf_size];
            let _ = reader.read_value(&buf);
            let _ = reader.read_row(&buf);
        }
    }
});

fn nth(data: &[u8], seed: &mut usize, max_val: usize) -> usize {
    if *seed >= data.len() { return 0; }
    let v = data[*seed] as usize;
    *seed += 1;
    v % max_val
}

fn primitive_kind(data: &[u8], seed: &mut usize) -> WireKind {
    let kinds = [
        WireKind::I8, WireKind::I16, WireKind::I32, WireKind::I64,
        WireKind::U8, WireKind::U16, WireKind::U32, WireKind::U64,
        WireKind::F16, WireKind::F32, WireKind::F64, WireKind::Bool,
    ];
    let idx = nth(data, seed, kinds.len());
    kinds[idx]
}

fn build_inodes(mode: u8, data: &[u8], seed: &mut usize) -> Vec<Inode> {
    match mode {
        0 => build_nested_primitive_list(data, seed),
        1 => build_list_of_groups(data, seed),
        2 => build_group_of_lists(data, seed),
        3 => build_deeply_nested(data, seed),
        4 => build_option_group_list(data, seed),
        5 => build_group_of_groups(data, seed),
        6 => build_nested_string_list(data, seed),
        7 => build_mixed_primitive_list(data, seed),
        8 => build_nested_bool_list(data, seed),
        9 => build_list_of_option_group(data, seed),
        10 => build_nested_fixed_list(data, seed),
        _ => build_random_hybrid(data, seed),
    }
}

fn make_primitive(kind: WireKind, nesting: u8) -> Inode {
    Inode {
        kind,
        kind_meta: KindMeta::Primitive { nesting, data_offset: 0, topology_offset: 0, fixed_size: 0, nullable_bit_index: 0, group: false, group_nest: false },
    }
}

fn build_nested_primitive_list(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let depth = (nth(data, seed, 4) + 1) as u8;
    let kind = primitive_kind(data, seed);
    vec![make_primitive(kind, depth)]
}

fn build_list_of_groups(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let child_count = (nth(data, seed, 5) + 1) as u8;
    let child_kinds = [WireKind::I32, WireKind::U64, WireKind::F32, WireKind::Bool, WireKind::Str];
    let mut inodes = vec![Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
 count: child_count as u16, nesting: 1, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
    }];
    for _ in 0..child_count {
        let kind = child_kinds[nth(data, seed, child_kinds.len())];
        inodes.push(make_primitive(kind, 0));
    }
    inodes
}

fn build_group_of_lists(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let child_count = (nth(data, seed, 4) + 1) as u8;
    let mut inodes = vec![Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
 count: child_count as u16, nesting: 0, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
    }];
    for _ in 0..child_count {
        let kind = primitive_kind(data, seed);
        inodes.push(make_primitive(kind, 1));
    }
    inodes
}

fn build_deeply_nested(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let depth = nth(data, seed, 6) as u8;
    let kinds = [WireKind::I8, WireKind::I16, WireKind::I32, WireKind::I64, WireKind::U8,
                 WireKind::U16, WireKind::U32, WireKind::U64, WireKind::F32, WireKind::F64,
                 WireKind::Bool, WireKind::Timestamp, WireKind::Str, WireKind::I32];
    let kind = kinds[nth(data, seed, kinds.len())];
    vec![make_primitive(kind, depth)]
}

fn build_option_group_list(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let child_count = (nth(data, seed, 3) + 1) as u8;
    let mut inodes = vec![Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
 count: child_count as u16, nesting: 1, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
    }];
    let kinds = [WireKind::OptionI32, WireKind::OptionU64, WireKind::I64, WireKind::OptionF32];
    for _ in 0..child_count {
        let kind = kinds[nth(data, seed, kinds.len())];
        inodes.push(make_primitive(kind, 0));
    }
    inodes
}

fn build_group_of_groups(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let inner_count = (nth(data, seed, 3) + 1) as u8;
    let mut inodes = vec![Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
 count: inner_count as u16, nesting: 1, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
    }];
    let leaf_kinds = [WireKind::U8, WireKind::U16, WireKind::U32, WireKind::U64];
    for _ in 0..inner_count {
        let inner_count2 = (nth(data, seed, 3) + 1) as u8;
        inodes.push(Inode {
            kind: WireKind::Group,
            kind_meta: KindMeta::Group {
 count: inner_count2 as u16, nesting: 0, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
        });
        for _ in 0..inner_count2 {
            let kind = leaf_kinds[nth(data, seed, leaf_kinds.len())];
            inodes.push(make_primitive(kind, 0));
        }
    }
    inodes
}

fn build_nested_string_list(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let depth = nth(data, seed, 4) as u8;
    vec![make_primitive(WireKind::Str, depth)]
}

fn build_mixed_primitive_list(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let num_fields = (nth(data, seed, 4) + 1) as usize;
    let kinds = [WireKind::I32, WireKind::U64, WireKind::F32, WireKind::Bool, WireKind::OptionI32, WireKind::OptionU64, WireKind::OptionF64];
    let mut inodes = vec![];
    for i in 0..num_fields {
        let kind = kinds[nth(data, seed, kinds.len())];
        let nesting = if i == 0 { 0 } else { nth(data, seed, 3) as u8 };
        inodes.push(make_primitive(kind, nesting));
    }
    inodes
}

fn build_nested_bool_list(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let depth = nth(data, seed, 5) as u8;
    vec![make_primitive(WireKind::Bool, depth)]
}

fn build_list_of_option_group(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let child_count = (nth(data, seed, 4) + 1) as u8;
    let mut inodes = vec![Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
 count: child_count as u16, nesting: 1, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
    }];
    for _ in 0..child_count {
        let kind = match nth(data, seed, 5) {
            0 => WireKind::OptionI32, 1 => WireKind::OptionU64, 2 => WireKind::OptionF32,
            3 => WireKind::I64, _ => WireKind::Bool,
        };
        inodes.push(make_primitive(kind, 0));
    }
    inodes
}

fn build_nested_fixed_list(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    if *seed >= data.len() { return vec![]; }
    let depth = (nth(data, seed, 3) + 1) as u8;
    let kinds = [WireKind::I8, WireKind::I16, WireKind::I32, WireKind::I64, WireKind::U8,
                 WireKind::U16, WireKind::U32, WireKind::U64, WireKind::F16, WireKind::F32,
                 WireKind::F64, WireKind::Bool, WireKind::I32];
    let kind = kinds[nth(data, seed, kinds.len())];
    let fixed_size = ((nth(data, seed, 16) as u16) + 1).min(16);
    vec![Inode {
        kind,
        kind_meta: KindMeta::Primitive { nesting: depth, data_offset: 0, topology_offset: 0, fixed_size, nullable_bit_index: 0, group: false, group_nest: false },
    }]
}

fn build_random_hybrid(data: &[u8], seed: &mut usize) -> Vec<Inode> {
    let mut inodes = vec![];
    let total = data.len().saturating_sub(*seed).min(10);
    let group_kinds = [WireKind::Group, WireKind::OptionGroup];
    let leaf_kinds = [WireKind::I32, WireKind::U64, WireKind::F32, WireKind::Bool, WireKind::Str, WireKind::Bytes, WireKind::OptionI32, WireKind::OptionU64];

    for _ in 0..total {
        let is_group = nth(data, seed, 5) == 0;
        if is_group {
            let kind = group_kinds[nth(data, seed, group_kinds.len())];
            let child_count = (nth(data, seed, 3) + 1) as u8;
            let nesting = nth(data, seed, 2) as u8;
            inodes.push(Inode {
                kind,
                kind_meta: KindMeta::Group {
 count: child_count as u16, nesting, topology_offset: 0,
 nullable_bit_index: 0,
 group: false,
 group_nest: false,
},
            });
            for _ in 0..child_count as usize {
                let ck = leaf_kinds[nth(data, seed, leaf_kinds.len())];
                inodes.push(make_primitive(ck, 0));
            }
        } else {
            let kind = leaf_kinds[nth(data, seed, leaf_kinds.len())];
            let nesting = nth(data, seed, 3) as u8;
            inodes.push(make_primitive(kind, nesting));
        }
    }
    inodes
}
