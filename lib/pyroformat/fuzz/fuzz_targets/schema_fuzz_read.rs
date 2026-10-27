//! Fuzz target: valid schema + value round-trip, then garbage-data reads.
//!
//! This target:
//! 1. Constructs a valid `PyroSchema` from fuzz input (hand-shaped modes plus
//!    a recursive random-schema mode), in either structure word (`U16`/`U32`).
//! 2. Flattens it with `SchemaParts::from_schema` and asserts the inode table
//!    survives `SchemaParts::encode`/`decode`.
//! 3. Generates a `PyroValue` matching the whole schema and writes it with the
//!    value path (`ValueWriter::new` -> `ingest` -> `write` into a
//!    `LocalMemSink`).
//! 4. Reads the buffer back with `RowReader` and asserts the round trip is
//!    lossless (modulo `PrimitiveList` vs `List` representation).
//! 5. Corrupts the buffer in various ways (bit flips, truncation, offset
//!    mangling, padding injection, oversized/tiny buffers, ...) and feeds each
//!    variant through `RowReader::read_value` / `read_row`.
//!
//! Panics anywhere — including in the reader on corrupted payloads — are
//! intentional: they are bugs to be found.

#![no_main]

use std::borrow::Cow;

use libfuzzer_sys::fuzz_target;
use pyroformat::{
    Map, PyroRow, PyroValue,
    buf::{LocalMemSink, PyroSink},
    schema::{
        FieldNames, PrimitiveDataType, PyroField, PyroSchema, PyroType, SchemaParts, Word,
        schema_hash,
    },
    value::{PrimitiveValueList, Time},
    wire::{value_readers::RowReader, value_writers::ValueWriter},
};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Low nibble: mode. High bit: structure word.
    let mode = (data[0] & 0x0F) % 11;
    let word = if data[0] & 0x80 != 0 { Word::U16 } else { Word::U32 };
    let mut seed = 1;

    let schema = match mode {
        0 => fuzz_simple_primitives(data, &mut seed),
        1 => fuzz_groups_and_lists(data, &mut seed),
        2 => fuzz_nested_structures(data, &mut seed),
        3 => fuzz_optionals(data, &mut seed),
        4 => fuzz_strings_and_bytes(data, &mut seed),
        5 => fuzz_maps(data, &mut seed),
        6 => fuzz_mixed_complex(data, &mut seed),
        7 => fuzz_large_payloads(data, &mut seed),
        8 => fuzz_schema_roundtrip(data, &mut seed),
        9 => fuzz_edge_schema(data, &mut seed),
        _ => fuzz_random_schema(data, &mut seed),
    };
    if let Some(mut schema) = schema {
        schema.set_word(word);
        run_case(&schema, data, &mut seed);
    }
});

// ─── helpers ────────────────────────────────────────────────────────────────

fn pick<T: Clone>(data: &[u8], seed: &mut usize, options: &[T]) -> T {
    if *seed >= data.len() || options.is_empty() {
        return options[0].clone();
    }
    let v = data[*seed] as usize;
    *seed += 1;
    options[v % options.len()].clone()
}

fn pick_int(data: &[u8], seed: &mut usize, max: usize) -> usize {
    if *seed >= data.len() || max == 0 {
        return 0;
    }
    let v = data[*seed] as usize;
    *seed += 1;
    v % max
}

/// Next `N` fuzz bytes (zero-filled once the input runs out).
fn take<const N: usize>(data: &[u8], seed: &mut usize) -> [u8; N] {
    let mut out = [0u8; N];
    for b in out.iter_mut() {
        if *seed < data.len() {
            *b = data[*seed];
            *seed += 1;
        }
    }
    out
}

fn coin(data: &[u8], seed: &mut usize) -> bool {
    pick_int(data, seed, 2) == 0
}

fn owned_names(names: &[&str]) -> FieldNames<'static> {
    FieldNames::Owned(names.iter().map(|s| Cow::Owned(s.to_string())).collect())
}

// ─── corruption helpers ─────────────────────────────────────────────────────

/// Flip random bits in the buffer.
fn flip_bits(buf: &mut [u8], data: &[u8], seed: &mut usize) {
    if buf.is_empty() {
        return;
    }
    let num_flips = pick_int(data, seed, buf.len().max(1)).min(buf.len() * 2);
    for _ in 0..num_flips {
        let pos = pick_int(data, seed, buf.len());
        let bit = pick_int(data, seed, 8);
        buf[pos] ^= 1 << bit;
    }
}

/// Truncate the buffer to a random size.
fn truncate_buf(buf: &mut Vec<u8>, data: &[u8], seed: &mut usize) {
    if buf.is_empty() {
        return;
    }
    let trunc = pick_int(data, seed, buf.len());
    buf.truncate(trunc);
}

/// Pad the buffer with garbage bytes.
fn pad_garbage(buf: &mut Vec<u8>, data: &[u8], seed: &mut usize) {
    let extra = pick_int(data, seed, 256);
    for _ in 0..extra {
        buf.push(data[*seed % data.len()]);
        *seed += 1;
    }
}

/// Shuffle a slice of the buffer.
fn shuffle_slice(buf: &mut [u8], data: &[u8], seed: &mut usize, start: usize, end: usize) {
    if start >= end || end > buf.len() {
        return;
    }
    let len = end - start;
    let num_swaps = pick_int(data, seed, len * 4);
    for _ in 0..num_swaps {
        let i = start + pick_int(data, seed, len);
        let j = start + pick_int(data, seed, len);
        buf.swap(i, j);
    }
}

/// Set the buffer to a repeating pattern.
fn fill_pattern(buf: &mut [u8], pattern: usize) {
    match pattern % 4 {
        0 => buf.fill(0),
        1 => buf.fill(0xFF),
        2 => buf.fill(0xAA),
        _ => buf.fill(0x55),
    }
}

/// Corrupt offset pointers by modifying bytes that might be part of u16/u32 offsets.
fn corrupt_offsets(buf: &mut [u8], data: &[u8], seed: &mut usize) {
    let num_corrupts = pick_int(data, seed, buf.len().min(32));
    for _ in 0..num_corrupts {
        if buf.is_empty() {
            return;
        }
        let pos = pick_int(data, seed, buf.len());
        buf[pos] = buf[pos].wrapping_add(1);
    }
}

// ─── schema builders (one per mode) ─────────────────────────────────────────

const ALL_PRIMS: [PrimitiveDataType; 13] = [
    PrimitiveDataType::I8,
    PrimitiveDataType::I16,
    PrimitiveDataType::I32,
    PrimitiveDataType::I64,
    PrimitiveDataType::U8,
    PrimitiveDataType::U16,
    PrimitiveDataType::U32,
    PrimitiveDataType::U64,
    PrimitiveDataType::F16,
    PrimitiveDataType::F32,
    PrimitiveDataType::F64,
    PrimitiveDataType::Bool,
    PrimitiveDataType::Timestamp,
];

fn fuzz_simple_primitives(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }
    let num_fields = pick_int(data, seed, 6) + 1;
    let fields = (0..num_fields)
        .map(|i| {
            PyroField::new(
                format!("f{}", i),
                PyroType::PrimitiveScalar(pick(data, seed, &ALL_PRIMS)),
            )
        })
        .collect();
    Some(PyroSchema::new(fields))
}

fn fuzz_groups_and_lists(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let mode = pick_int(data, seed, 4);
    let mut fields = Vec::new();

    match mode {
        0 => {
            // Group with primitive fields
            let child_count = pick_int(data, seed, 5) + 1;
            let kinds = [
                PrimitiveDataType::U8,
                PrimitiveDataType::U32,
                PrimitiveDataType::F32,
                PrimitiveDataType::I64,
                PrimitiveDataType::Bool,
            ];
            let child_fields: Vec<PyroField> = (0..child_count)
                .map(|i| {
                    PyroField::new(
                        format!("c{}", i),
                        PyroType::PrimitiveScalar(pick(data, seed, &kinds)),
                    )
                })
                .collect();
            fields.push(PyroField::group("group0", child_fields));
        }
        1 => {
            // List of primitives
            let kinds = [
                PrimitiveDataType::U8,
                PrimitiveDataType::I16,
                PrimitiveDataType::U32,
                PrimitiveDataType::F64,
                PrimitiveDataType::Bool,
            ];
            let kind = pick(data, seed, &kinds);
            fields.push(PyroField::new("list0", PyroType::PrimitiveList(kind)));
        }
        2 => {
            // A list next to a scalar
            let kinds = [
                PrimitiveDataType::U8,
                PrimitiveDataType::I32,
                PrimitiveDataType::F32,
            ];
            fields.push(PyroField::new(
                "items",
                PyroType::PrimitiveList(pick(data, seed, &kinds)),
            ));
            fields.push(PyroField::new(
                "count",
                PyroType::PrimitiveScalar(PrimitiveDataType::U32),
            ));
        }
        _ => {
            // Nested group
            let inner_fields = vec![
                PyroField::new("x", PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
                PyroField::new("y", PyroType::PrimitiveScalar(PrimitiveDataType::F64)),
            ];
            let inner_group = PyroField::group("inner", inner_fields);
            let outer_fields = vec![
                PyroField::new("a", PyroType::PrimitiveScalar(PrimitiveDataType::U8)),
                inner_group,
            ];
            fields.push(PyroField::group("outer", outer_fields));
        }
    }

    Some(PyroSchema::new(fields))
}

fn fuzz_nested_structures(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let depth = pick_int(data, seed, 4) + 1;
    let kind = pick(
        data,
        seed,
        &[
            PrimitiveDataType::U32,
            PrimitiveDataType::F64,
            PrimitiveDataType::I8,
            PrimitiveDataType::Bool,
        ],
    );

    // Build List<List<...primitive...>>
    let mut inner = PyroType::PrimitiveScalar(kind);
    for _ in 0..depth {
        inner = PyroType::List(inner.into());
    }

    Some(PyroSchema::new(vec![
        PyroField::new("deep", inner),
        PyroField::new("flat", PyroType::PrimitiveScalar(PrimitiveDataType::U8)),
    ]))
}

fn fuzz_optionals(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let num_fields = pick_int(data, seed, 6) + 1;
    let mut fields = Vec::new();
    for i in 0..num_fields {
        let kind = pick(data, seed, &ALL_PRIMS);
        let ty = if i % 3 == 0 {
            // Make some required, some optional
            PyroType::PrimitiveScalar(kind)
        } else {
            PyroType::OptionPrimitiveScalar(kind)
        };
        fields.push(PyroField::new(format!("f{}", i), ty));
    }

    Some(PyroSchema::new(fields))
}

fn fuzz_strings_and_bytes(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let mut fields = Vec::new();
    match pick_int(data, seed, 3) {
        0 => fields.push(PyroField::new("messages", PyroType::List(PyroType::Str.into()))),
        1 => {
            fields.push(PyroField::new("name", PyroType::Str));
            fields.push(PyroField::new("data", PyroType::Bytes));
        }
        _ => fields.push(PyroField::new(
            "blobs",
            PyroType::List(PyroType::List(PyroType::Bytes.into()).into()),
        )),
    }

    Some(PyroSchema::new(fields))
}

fn fuzz_maps(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let field = if coin(data, seed) {
        // List<Map<Str, U32>>
        PyroField::new(
            "entries",
            PyroType::List(
                PyroType::Map {
                    key: PyroType::Str.into(),
                    value: PyroType::PrimitiveScalar(PrimitiveDataType::U32).into(),
                }
                .into(),
            ),
        )
    } else {
        // Map<Str, I64>
        PyroField::new(
            "lookup",
            PyroType::Map {
                key: PyroType::Str.into(),
                value: PyroType::PrimitiveScalar(PrimitiveDataType::I64).into(),
            },
        )
    };

    Some(PyroSchema::new(vec![field]))
}

fn fuzz_mixed_complex(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let mut fields = Vec::new();

    // Simple fields
    fields.push(PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::U64)));
    fields.push(PyroField::new("name", PyroType::Str));

    // Optional group
    let group_fields = vec![
        PyroType::PrimitiveScalar(PrimitiveDataType::I32),
        PyroType::PrimitiveScalar(PrimitiveDataType::F64),
        PyroType::OptionPrimitiveScalar(PrimitiveDataType::U16),
    ];
    fields.push(PyroField::with_children(
        "optional_info",
        owned_names(&["f0", "f1", "f2"]),
        PyroType::OptionGroup(group_fields.into()),
    ));

    // List of groups
    let list_item_types = vec![
        PyroType::Str,
        PyroType::PrimitiveScalar(PrimitiveDataType::U8),
    ];
    fields.push(PyroField::with_children(
        "tags",
        owned_names(&["label", "priority"]),
        PyroType::List(PyroType::Group(list_item_types.into()).into()),
    ));

    // Map
    fields.push(PyroField::new(
        "metadata",
        PyroType::Map {
            key: PyroType::Str.into(),
            value: PyroType::PrimitiveScalar(PrimitiveDataType::F32).into(),
        },
    ));

    // Nested list
    fields.push(PyroField::new(
        "matrix",
        PyroType::List(
            PyroType::List(PyroType::PrimitiveScalar(PrimitiveDataType::F64).into()).into(),
        ),
    ));

    Some(PyroSchema::new(fields))
}

fn fuzz_large_payloads(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let kinds = [
        PrimitiveDataType::U8,
        PrimitiveDataType::U16,
        PrimitiveDataType::U32,
        PrimitiveDataType::U64,
        PrimitiveDataType::F32,
        PrimitiveDataType::F64,
    ];

    // Eight long lists: the value generator below sizes lists from fuzz
    // bytes, and `List` here lets it go big.
    let fields = (0..8)
        .map(|i| {
            let kind = pick(data, seed, &kinds);
            let ty = if i % 2 == 0 {
                PyroType::PrimitiveScalar(kind)
            } else {
                PyroType::PrimitiveList(kind)
            };
            PyroField::new(format!("f{}", i), ty)
        })
        .collect();

    Some(PyroSchema::new(fields))
}

/// The schema goes through `PyroSchema::encode`/`decode` before use.
fn fuzz_schema_roundtrip(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let kinds = [
        PrimitiveDataType::I8,
        PrimitiveDataType::I32,
        PrimitiveDataType::U64,
        PrimitiveDataType::F32,
        PrimitiveDataType::Bool,
    ];

    let num_fields = pick_int(data, seed, 5) + 1;
    let mut fields = Vec::new();
    for i in 0..num_fields {
        let kind = pick(data, seed, &kinds);
        let ty = if i % 4 == 0 {
            PyroType::OptionPrimitiveScalar(kind)
        } else {
            PyroType::PrimitiveScalar(kind)
        };
        fields.push(PyroField::new(format!("f{}", i), ty));
    }

    let schema = PyroSchema::new(fields);
    let encoded = schema.encode().ok()?;
    let decoded = PyroSchema::decode(&encoded).expect("decode of an encoded schema");
    assert_eq!(decoded.types(), schema.types(), "schema types changed over encode/decode");
    // `decoded` borrows `encoded`; rebuild an owned schema from its parts.
    let names: Vec<&str> = decoded.fields().map(|f| f.name()).collect();
    let fields = names
        .iter()
        .zip(decoded.types())
        .map(|(n, t)| PyroField::new(n.to_string(), t.clone()))
        .collect();
    Some(PyroSchema::new(fields))
}

fn fuzz_edge_schema(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }

    let mode = pick_int(data, seed, 7);
    let field = match mode {
        // Empty group
        0 => PyroField::group("empty", vec![]),
        // Single string in group
        1 => PyroField::group("single", vec![PyroField::new("s", PyroType::Str)]),
        // List of empty groups
        2 => PyroField::with_children(
            "empties",
            FieldNames::Owned(vec![]),
            PyroType::List(PyroType::Group(vec![].into()).into()),
        ),
        // Map with group value
        3 => PyroField::with_children(
            "lookup",
            owned_names(&["f0", "f1"]),
            PyroType::Map {
                key: PyroType::Str.into(),
                value: PyroType::Group(
                    vec![
                        PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                        PyroType::PrimitiveScalar(PrimitiveDataType::U64),
                    ]
                    .into(),
                )
                .into(),
            },
        ),
        // Fixed-size array
        4 => {
            let n = pick_int(data, seed, 8) + 1;
            PyroField::new(
                "fixed",
                PyroType::PrimitiveFixedList(pick(data, seed, &ALL_PRIMS), n),
            )
        }
        // Nullable fixed-size array in a list
        5 => {
            let n = pick_int(data, seed, 8) + 1;
            PyroField::new(
                "fixed_opts",
                PyroType::List(
                    PyroType::OptionPrimitiveFixedList(pick(data, seed, &ALL_PRIMS), n).into(),
                ),
            )
        }
        // Deeply nested lists
        _ => {
            let depth = pick_int(data, seed, 5) + 2;
            let mut inner = PyroType::PrimitiveScalar(PrimitiveDataType::U32);
            for _ in 0..depth {
                inner = PyroType::List(inner.into());
            }
            PyroField::new("deep", inner)
        }
    };

    Some(PyroSchema::new(vec![field]))
}

/// A recursively generated schema: groups, option groups, lists, maps,
/// fixed arrays, strings, bytes and scalars, nested up to a small depth.
fn fuzz_random_schema(data: &[u8], seed: &mut usize) -> Option<PyroSchema<'static>> {
    if *seed >= data.len() {
        return None;
    }
    let num_fields = pick_int(data, seed, 4) + 1;
    let fields = (0..num_fields)
        .map(|i| {
            let (ty, names) = random_type(data, seed, 0);
            PyroField::with_children(
                format!("f{}", i),
                FieldNames::Owned(names.into_iter().map(Cow::Owned).collect()),
                ty,
            )
        })
        .collect();
    Some(PyroSchema::new(fields))
}

/// A random type plus its flattened pre-order child names (the packing
/// `PyroField::with_children` expects: one entry per `name_count`).
fn random_type(data: &[u8], seed: &mut usize, depth: usize) -> (PyroType, Vec<String>) {
    // Past the depth budget (or out of input), only leaves.
    let choice = if depth >= 4 || *seed >= data.len() {
        pick_int(data, seed, 6)
    } else {
        pick_int(data, seed, 12)
    };
    match choice {
        0 | 1 => (PyroType::PrimitiveScalar(pick(data, seed, &ALL_PRIMS)), vec![]),
        2 => (PyroType::OptionPrimitiveScalar(pick(data, seed, &ALL_PRIMS)), vec![]),
        3 => (PyroType::Str, vec![]),
        4 => (PyroType::Bytes, vec![]),
        5 => (PyroType::PrimitiveList(pick(data, seed, &ALL_PRIMS)), vec![]),
        6 => {
            let n = pick_int(data, seed, 4) + 1;
            let k = pick(data, seed, &ALL_PRIMS);
            if coin(data, seed) {
                (PyroType::PrimitiveFixedList(k, n), vec![])
            } else {
                (PyroType::OptionPrimitiveFixedList(k, n), vec![])
            }
        }
        7 | 8 => {
            let (inner, names) = random_type(data, seed, depth + 1);
            (PyroType::List(inner.into()), names)
        }
        9 | 10 => {
            let n = pick_int(data, seed, 4) + usize::from(choice == 10);
            let mut types = Vec::with_capacity(n);
            let mut names = Vec::new();
            for i in 0..n {
                let (t, sub) = random_type(data, seed, depth + 1);
                names.push(format!("c{}", i));
                names.extend(sub);
                types.push(t);
            }
            let ty = if choice == 9 {
                PyroType::Group(types.into())
            } else {
                PyroType::OptionGroup(types.into())
            };
            (ty, names)
        }
        _ => {
            let key = pick(
                data,
                seed,
                &[
                    PyroType::Str,
                    PyroType::PrimitiveScalar(PrimitiveDataType::U8),
                    PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                ],
            );
            let (value, names) = random_type(data, seed, depth + 1);
            (
                PyroType::Map {
                    key: key.into(),
                    value: value.into(),
                },
                names,
            )
        }
    }
}

// ─── write, round-trip, corrupt ─────────────────────────────────────────────

fn run_case(schema: &PyroSchema<'_>, data: &[u8], seed: &mut usize) {
    // Schemas the layout rejects (too deep, too many inodes, ...) are fine.
    let Ok(parts) = SchemaParts::from_schema(schema) else {
        return;
    };
    assert_eq!(parts.meta, schema.meta());

    // The inode table (with its meta slot) survives encode/decode.
    let encoded = parts.encode(true);
    let decoded = SchemaParts::decode(&encoded).expect("SchemaParts::decode of encoded parts");
    assert_eq!(decoded, parts, "SchemaParts changed over encode/decode");

    let Ok(vw) = ValueWriter::new(schema) else {
        return;
    };
    assert_eq!(vw.inodes(), &parts.inodes[..]);
    let val = make_value_for_schema(data, seed, schema);
    let vw = vw
        .ingest(&val)
        .unwrap_or_else(|e| panic!("ingest of a schema-shaped value failed: {e:?}\nschema: {schema:?}\nvalue: {val:?}"));

    let mut sink = LocalMemSink::new(vw.fixed_len(), schema_hash(&parts.meta, &parts.inodes));
    if sink.reallocate_sync(vw.variable_len()).is_err() {
        return;
    }
    // `OffsetOverflow` (a value too big for a `U16` message) is legitimate.
    if vw.write(&mut sink).is_err() {
        return;
    }
    let buf = sink.payload().to_vec();

    let reader = RowReader::from_parts(parts.meta, parts.inodes.clone(), &parts.field)
        .expect("RowReader for a schema ValueWriter accepted");
    let via_schema = RowReader::new(schema).expect("RowReader::new for the same schema");
    assert_eq!(via_schema.inodes(), reader.inodes());
    assert_eq!(via_schema.inode_meta(), reader.inode_meta());

    // Valid read: lossless round trip.
    let restored = reader
        .read_value(&buf)
        .unwrap_or_else(|e| panic!("read of a freshly written buffer failed: {e:?}\nschema: {schema:?}\nvalue: {val:?}"));
    assert_eq!(
        normalize(&val),
        normalize(&restored),
        "value round trip mismatch\nschema: {schema:?}"
    );
    reader.read_row(&buf).expect("read_row of a freshly written buffer");

    corrupt_and_read(&reader, &buf, data, seed);
}

fn read_both(reader: &RowReader, buf: &[u8]) {
    let _ = reader.read_value(buf);
    let _ = reader.read_row(buf);
}

fn corrupt_and_read(reader: &RowReader, buf: &[u8], data: &[u8], seed: &mut usize) {
    let len = buf.len();

    // Pattern fills
    for pattern in 0..4 {
        let mut c = buf.to_vec();
        fill_pattern(&mut c, pattern);
        read_both(reader, &c);
    }

    // Bit flips
    let mut c = buf.to_vec();
    flip_bits(&mut c, data, seed);
    read_both(reader, &c);

    // Truncation
    let mut c = buf.to_vec();
    truncate_buf(&mut c, data, seed);
    read_both(reader, &c);

    // Offset corruption
    let mut c = buf.to_vec();
    corrupt_offsets(&mut c, data, seed);
    read_both(reader, &c);

    // Shuffle
    let mut c = buf.to_vec();
    let end = c.len().min(1024);
    shuffle_slice(&mut c, data, seed, 0, end);
    read_both(reader, &c);

    // Padding
    let mut c = buf.to_vec();
    pad_garbage(&mut c, data, seed);
    read_both(reader, &c);

    // Oversized
    let mut big = vec![0u8; len.saturating_add(262144)];
    big[..len].copy_from_slice(buf);
    read_both(reader, &big);

    // Tiny
    if !buf.is_empty() {
        read_both(reader, &buf[..1]);
    }

    // Progressive truncation
    for t in 1..=5 {
        let target = len * t / 5;
        if target < len {
            read_both(reader, &buf[..target]);
        }
    }
}

// ─── value builders ─────────────────────────────────────────────────────────

/// A value for the whole schema, in the shape `ValueWriter::ingest` takes:
/// a lone top-level field's value directly, otherwise a `Group` keyed by
/// field name.
fn make_value_for_schema(data: &[u8], seed: &mut usize, schema: &PyroSchema<'_>) -> PyroValue<'static> {
    let mut row = PyroRow::new();
    let mut single = None;
    for field in schema.fields() {
        let names: Vec<&str> = (0..field.child_names_len()).map(|i| field.child_name(i)).collect();
        let v = make_value(data, seed, field.datatype(), &names);
        if schema.num_fields() == 1 {
            single = Some(v);
        } else {
            row.insert(field.name().to_string(), v);
        }
    }
    single.unwrap_or(PyroValue::Group(row))
}

/// List lengths: mostly small, occasionally large.
fn list_len(data: &[u8], seed: &mut usize) -> usize {
    match pick_int(data, seed, 16) {
        15 => pick_int(data, seed, 256) * 4,
        n => n % 5,
    }
}

/// A value of type `ty`; `names` is `ty`'s flattened pre-order child-name
/// slice (exactly `ty.name_count()` long).
fn make_value(data: &[u8], seed: &mut usize, ty: &PyroType, names: &[&str]) -> PyroValue<'static> {
    match ty {
        PyroType::Unknown => PyroValue::Null,
        PyroType::PrimitiveScalar(k) => make_scalar(data, seed, *k),
        PyroType::OptionPrimitiveScalar(k) => {
            if coin(data, seed) {
                make_scalar(data, seed, *k)
            } else {
                PyroValue::Null
            }
        }
        PyroType::Str => {
            let n = pick_int(data, seed, 12);
            let bytes: Vec<u8> = (0..n).map(|_| take::<1>(data, seed)[0]).collect();
            PyroValue::Str(Cow::Owned(String::from_utf8_lossy(&bytes).into_owned()))
        }
        PyroType::Bytes => {
            let n = pick_int(data, seed, 12);
            PyroValue::Bytes(Cow::Owned((0..n).map(|_| take::<1>(data, seed)[0]).collect()))
        }
        PyroType::PrimitiveList(k) => {
            let n = list_len(data, seed);
            make_prim_list(data, seed, *k, n)
        }
        PyroType::PrimitiveFixedList(k, n) => make_prim_list(data, seed, *k, *n),
        PyroType::OptionPrimitiveFixedList(k, n) => {
            if coin(data, seed) {
                make_prim_list(data, seed, *k, *n)
            } else {
                PyroValue::Null
            }
        }
        PyroType::List(inner) => {
            let n = list_len(data, seed);
            PyroValue::List((0..n).map(|_| make_value(data, seed, inner, names)).collect())
        }
        PyroType::Group(children) => PyroValue::Group(make_row(data, seed, children, names)),
        PyroType::OptionGroup(children) => {
            if pick_int(data, seed, 4) == 0 {
                PyroValue::Null
            } else {
                PyroValue::Group(make_row(data, seed, children, names))
            }
        }
        PyroType::Map { key, value } => {
            let kn = key.name_count();
            let (key_names, value_names) = names.split_at(kn);
            let n = list_len(data, seed);
            let entries: Vec<_> = (0..n)
                .map(|_| {
                    (
                        make_value(data, seed, key, key_names),
                        make_value(data, seed, value, value_names),
                    )
                })
                .collect();
            PyroValue::Map(Map::from(entries))
        }
    }
}

fn make_row(data: &[u8], seed: &mut usize, children: &[PyroType], names: &[&str]) -> PyroRow<'static> {
    let mut row = PyroRow::new();
    let mut off = 0;
    for child in children {
        let nc = child.name_count();
        let name = names[off];
        let v = make_value(data, seed, child, &names[off + 1..off + 1 + nc]);
        row.insert(name.to_string(), v);
        off += 1 + nc;
    }
    row
}

/// A list of `n` `k`s, as either a `List` of scalars or a typed
/// `PrimitiveList` — the writer takes both.
fn make_prim_list(data: &[u8], seed: &mut usize, k: PrimitiveDataType, n: usize) -> PyroValue<'static> {
    let items: Vec<PyroValue<'static>> = (0..n).map(|_| make_scalar(data, seed, k)).collect();
    if k == PrimitiveDataType::Bool || coin(data, seed) {
        return PyroValue::List(items);
    }
    macro_rules! typed {
        ($variant:ident) => {
            PrimitiveValueList::$variant(Cow::Owned(
                items
                    .iter()
                    .map(|v| match v {
                        PyroValue::$variant(x) => *x,
                        _ => unreachable!(),
                    })
                    .collect(),
            ))
        };
    }
    PyroValue::PrimitiveList(match k {
        PrimitiveDataType::I8 => typed!(I8),
        PrimitiveDataType::I16 => typed!(I16),
        PrimitiveDataType::I32 => typed!(I32),
        PrimitiveDataType::I64 => typed!(I64),
        PrimitiveDataType::U8 => typed!(U8),
        PrimitiveDataType::U16 => typed!(U16),
        PrimitiveDataType::U32 => typed!(U32),
        PrimitiveDataType::U64 => typed!(U64),
        PrimitiveDataType::F16 => typed!(F16),
        PrimitiveDataType::F32 => typed!(F32),
        PrimitiveDataType::F64 => typed!(F64),
        PrimitiveDataType::Timestamp => typed!(Timestamp),
        PrimitiveDataType::Bool => unreachable!(),
    })
}

/// A scalar of kind `kind` from the next fuzz bytes. NaNs are replaced
/// (`NaN != NaN` would fail the round-trip assertion spuriously).
fn make_scalar(data: &[u8], seed: &mut usize, kind: PrimitiveDataType) -> PyroValue<'static> {
    match kind {
        PrimitiveDataType::Bool => PyroValue::Bool(take::<1>(data, seed)[0] & 1 == 1),
        PrimitiveDataType::I8 => PyroValue::I8(i8::from_le_bytes(take(data, seed))),
        PrimitiveDataType::I16 => PyroValue::I16(i16::from_le_bytes(take(data, seed))),
        PrimitiveDataType::I32 => PyroValue::I32(i32::from_le_bytes(take(data, seed))),
        PrimitiveDataType::I64 => PyroValue::I64(i64::from_le_bytes(take(data, seed))),
        PrimitiveDataType::U8 => PyroValue::U8(u8::from_le_bytes(take(data, seed))),
        PrimitiveDataType::U16 => PyroValue::U16(u16::from_le_bytes(take(data, seed))),
        PrimitiveDataType::U32 => PyroValue::U32(u32::from_le_bytes(take(data, seed))),
        PrimitiveDataType::U64 => PyroValue::U64(u64::from_le_bytes(take(data, seed))),
        PrimitiveDataType::F16 => {
            let v = half::f16::from_bits(u16::from_le_bytes(take(data, seed)));
            PyroValue::F16(if v.is_nan() { half::f16::ZERO } else { v })
        }
        PrimitiveDataType::F32 => {
            let v = f32::from_le_bytes(take(data, seed));
            PyroValue::F32(if v.is_nan() { 0.0 } else { v })
        }
        PrimitiveDataType::F64 => {
            let v = f64::from_le_bytes(take(data, seed));
            PyroValue::F64(if v.is_nan() { 0.0 } else { v })
        }
        PrimitiveDataType::Timestamp => {
            PyroValue::Timestamp(Time(i64::from_le_bytes(take(data, seed)) as i128))
        }
    }
}

/// Representation-independent form for comparison: typed `PrimitiveList`s
/// become `List`s of scalars, and row keys become owned.
fn normalize(v: &PyroValue<'_>) -> PyroValue<'static> {
    fn scalars<T: Copy>(xs: &[T], f: impl Fn(T) -> PyroValue<'static>) -> PyroValue<'static> {
        PyroValue::List(xs.iter().map(|x| f(*x)).collect())
    }
    match v {
        PyroValue::PrimitiveList(pl) => match pl {
            PrimitiveValueList::Bool(bits) => {
                PyroValue::List(bits.iter().by_vals().map(PyroValue::Bool).collect())
            }
            PrimitiveValueList::U8(x) => scalars(x, PyroValue::U8),
            PrimitiveValueList::U16(x) => scalars(x, PyroValue::U16),
            PrimitiveValueList::U32(x) => scalars(x, PyroValue::U32),
            PrimitiveValueList::U64(x) => scalars(x, PyroValue::U64),
            PrimitiveValueList::I8(x) => scalars(x, PyroValue::I8),
            PrimitiveValueList::I16(x) => scalars(x, PyroValue::I16),
            PrimitiveValueList::I32(x) => scalars(x, PyroValue::I32),
            PrimitiveValueList::I64(x) => scalars(x, PyroValue::I64),
            PrimitiveValueList::F16(x) => scalars(x, PyroValue::F16),
            PrimitiveValueList::F32(x) => scalars(x, PyroValue::F32),
            PrimitiveValueList::F64(x) => scalars(x, PyroValue::F64),
            PrimitiveValueList::Timestamp(x) => scalars(x, PyroValue::Timestamp),
        },
        PyroValue::List(items) => PyroValue::List(items.iter().map(normalize).collect()),
        PyroValue::Group(row) => {
            let mut out = PyroRow::new();
            for (k, v) in row.iter() {
                out.insert(k.to_string(), normalize(v));
            }
            PyroValue::Group(out)
        }
        PyroValue::Map(m) => {
            PyroValue::Map(m.iter().map(|(k, v)| (normalize(k), normalize(v))).collect())
        }
        other => other.clone().to_static(),
    }
}
