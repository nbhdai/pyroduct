//! The honest one — `docs/deep-ref-plan.md` §7: a full round trip, `encode`
//! then `pyro_read`, makes **exactly one** allocation: the message buffer.
//! The read side is borrowed views all the way down — scalars, `String`,
//! `Vec<numeric>`, `Vec<bool>`, `[T; N]`, nested structs, nested lists and
//! maps — so it adds none. This file is what stays honest as the code
//! changes, so every assertion here is a number actually measured against
//! the allocator, not a guess.
//!
//! Each round trip is measured warm: `encode` sizes its buffer from the
//! type's `max_variable()` high-water mark, which starts at zero, so the
//! very first encode of a type allocates a fixed-only buffer and then
//! grows it. A warm-up encode first raises the mark, as any second message
//! of the type would find it.

mod support;

use pyroformat::{Layout, Reader, Writer};
use support::alloc_counter::{AllocGuard, CountingAlloc};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc::new();

/// Allocations in one warm round trip of `data`: `encode`, then `pyro_read`
/// of the result. `check` sees the view once counting has stopped.
fn round_trip_allocs<T, F>(data: &T, check: F) -> usize
where
    T: Writer + Reader,
    F: for<'a> FnOnce(&T::Ref<'a>),
{
    drop(data.encode().unwrap());

    let g = AllocGuard::arm();
    let buf = data.encode().unwrap();
    let view = T::pyro_read(&buf).unwrap();
    let allocs = g.allocs();
    g.disarm();

    check(&view);
    allocs
}

// ===========================================================================
// Flat: scalars, String, Vec<numeric>, Vec<bool>, [T; N], nested struct
// ===========================================================================

#[derive(Layout, Reader, Writer, Debug, Clone, PartialEq)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Reader, Writer, Debug, Clone, PartialEq)]
struct ZeroAllocBudget {
    id: u32,
    active: bool,
    score: f64,
    name: String,
    numbers: Vec<u32>,
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
    matrix: [u32; 4],
    home: Address,
}

#[test]
fn round_trip_of_scalars_strings_slices_bools_fixed_and_nested_allocates_once() {
    let data = ZeroAllocBudget {
        id: 1,
        active: true,
        score: 2.5,
        name: "hello".to_string(),
        numbers: vec![1, 2, 3, 4, 5],
        flags: vec![true, false, true, true, false, false, true, true, false],
        matrix: [10, 20, 30, 40],
        home: Address {
            street: "1 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: 12345,
        },
    };
    let allocs = round_trip_allocs(&data, |view| {
        // Not just "didn't allocate" — actually the right, zero-copy values.
        assert_eq!(view.name, "hello");
        assert_eq!(view.numbers, &[1, 2, 3, 4, 5]);
        assert_eq!(
            view.flags.iter().by_vals().collect::<Vec<bool>>(),
            data.flags
        );
        assert_eq!(*view.matrix, [10, 20, 30, 40]);
        assert_eq!(view.home.street, "1 Main St");
    });

    assert_eq!(
        allocs, 1,
        "a round trip over scalars/String/Vec<numeric>/Vec<bool>/[T; N]/nested struct \
         should allocate only the message buffer -- got {allocs}"
    );
}

// ===========================================================================
// Nested: lists of lists, and maps, still borrow every column.
// ===========================================================================

#[derive(Layout, Reader, Writer, Debug, Clone, PartialEq)]
struct WithLabels {
    labels: Vec<String>,
}

#[test]
fn round_trip_of_vec_string_allocates_once() {
    // Vec<String> reads as a borrowed column: every `&str` slices the
    // buffer, and nothing collects them into a `Vec`. This used to cost
    // the reader 4 allocations, then 1 (the output `Vec<&str>`).
    //
    // Still measured rather than reasoned: a misaligned topology array
    // would add one, which is exactly the kind of thing worth seeing as a
    // diff.
    let data = WithLabels {
        labels: vec!["a".to_string(), "bb".to_string(), "ccc".to_string()],
    };
    let allocs = round_trip_allocs(&data, |view| {
        assert_eq!(view.labels, vec!["a", "bb", "ccc"]);
    });

    assert_eq!(
        allocs, 1,
        "a Vec<String> round trip should allocate only the message buffer; \
         see the comment above this test -- got {allocs}"
    );
}

#[derive(Layout, Reader, Writer, Debug, Clone, PartialEq)]
struct DeepVecs {
    matrix: Vec<Vec<u32>>,
}

#[test]
fn round_trip_of_vec_vec_numeric_allocates_once() {
    // Every inner list is a zero-copy borrow, the empty one costs nothing,
    // and the outer list is a view over the topology rather than a
    // `Vec<&[u32]>`. Was 4 on the read side, then 1.
    let data = DeepVecs {
        matrix: vec![vec![1, 2], vec![], vec![3]],
    };
    let allocs = round_trip_allocs(&data, |view| {
        assert_eq!(view.matrix, vec![&[1u32, 2][..], &[][..], &[3][..]]);
    });

    assert_eq!(
        allocs, 1,
        "a Vec<Vec<u32>> round trip should allocate only the message buffer -- got {allocs}"
    );
}

#[derive(Layout, Reader, Writer, Debug, Clone, PartialEq)]
struct WithStringMap {
    data: std::collections::BTreeMap<String, u32>,
}

#[test]
fn round_trip_of_string_key_map_allocates_once() {
    // MapRef over two borrowed columns: the values a `&[u32]` slice, the
    // keys a borrowed string column rather than a collected `Vec<&str>`
    // (which used to be the read side's one allocation).
    let mut data_map = std::collections::BTreeMap::new();
    data_map.insert("a".to_string(), 1u32);
    data_map.insert("bb".to_string(), 2u32);
    let data = WithStringMap { data: data_map };
    let allocs = round_trip_allocs(&data, |view| {
        assert_eq!(view.data.get(&"a"), Some(1));
        assert_eq!(view.data.get(&"bb"), Some(2));
    });

    assert_eq!(
        allocs, 1,
        "a Map<String, u32> round trip should allocate only the message buffer -- got {allocs}"
    );
}

#[derive(Layout, Reader, Writer, Debug, Clone, PartialEq)]
struct WithNumericMap {
    data: std::collections::BTreeMap<u32, u32>,
}

#[test]
fn round_trip_of_all_numeric_map_allocates_once() {
    // MapRef<'a, u32, u32>: both columns are borrowed slices straight off
    // the wire -- the fully zero-copy case D2 was designed for.
    let mut data_map = std::collections::BTreeMap::new();
    data_map.insert(1u32, 10u32);
    data_map.insert(2u32, 20u32);
    let data = WithNumericMap { data: data_map };
    let allocs = round_trip_allocs(&data, |view| {
        assert_eq!(view.data.get(&1), Some(10));
    });

    assert_eq!(
        allocs, 1,
        "a Map<u32, u32> round trip should allocate only the message buffer -- got {allocs}"
    );
}
