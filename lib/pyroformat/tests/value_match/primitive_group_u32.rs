use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_group_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! group_pair_test {
    (
        $test_name:ident,
        $group_ty:ident,
        $x_ty:ident,
        $x_val:expr,
        $y_ty:ident,
        $y_val:expr,
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = u32)]
        struct $group_ty {
            x: $x_ty,
            y: $y_ty,
        }

        #[test]
        fn $test_name() {
            let data = $group_ty {
                x: $x_val,
                y: $y_val,
            };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

group_pair_test!(
    value_match_group_pair_bool_bool,
    GroupBoolBool,
    bool,
    true,
    bool,
    true,
    GROUP_PAIR_BOOL_BOOL,
    GROUP_PAIR_BOOL_BOOL_INODES
);

// Field declaration order matters for `INODES` (it mirrors the
// source schema's field-declaration order, independent of the bucket-sort
// that only governs `data_offset` placement — see this file's module doc),
// so `x`/`y` below are ordered to match `GROUP_PAIR_BOOL_U16`'s own schema
// (which declares `y: U16` before `x: Bool`), not this const's own name
// order.
group_pair_test!(
    value_match_group_pair_bool_u16,
    GroupBoolU16,
    u16,
    54321u16,
    bool,
    true,
    GROUP_PAIR_BOOL_U16,
    GROUP_PAIR_BOOL_U16_INODES
);

group_pair_test!(
    value_match_group_pair_bool_u32,
    GroupBoolU32,
    bool,
    true,
    u32,
    123456u32,
    GROUP_PAIR_BOOL_U32,
    GROUP_PAIR_BOOL_U32_INODES
);

group_pair_test!(
    value_match_group_pair_bool_u64,
    GroupBoolU64,
    u64,
    12345678901234567890u64,
    bool,
    true,
    GROUP_PAIR_BOOL_U64,
    GROUP_PAIR_BOOL_U64_INODES
);

group_pair_test!(
    value_match_group_pair_bool_timestamp,
    GroupBoolTimestamp,
    bool,
    true,
    Time,
    Time(999_999_999),
    GROUP_PAIR_BOOL_TIMESTAMP,
    GROUP_PAIR_BOOL_TIMESTAMP_INODES
);

group_pair_test!(
    value_match_group_pair_u16_u16,
    GroupU16U16,
    u16,
    54321u16,
    u16,
    54321u16,
    GROUP_PAIR_U16_U16,
    GROUP_PAIR_U16_U16_INODES
);

group_pair_test!(
    value_match_group_pair_u16_u32,
    GroupU16U32,
    u16,
    54321u16,
    u32,
    123456u32,
    GROUP_PAIR_U16_U32,
    GROUP_PAIR_U16_U32_INODES
);

group_pair_test!(
    value_match_group_pair_u16_u64,
    GroupU16U64,
    u64,
    12345678901234567890u64,
    u16,
    54321u16,
    GROUP_PAIR_U16_U64,
    GROUP_PAIR_U16_U64_INODES
);

group_pair_test!(
    value_match_group_pair_u16_timestamp,
    GroupU16Timestamp,
    u16,
    54321u16,
    Time,
    Time(999_999_999),
    GROUP_PAIR_U16_TIMESTAMP,
    GROUP_PAIR_U16_TIMESTAMP_INODES
);

group_pair_test!(
    value_match_group_pair_u32_u32,
    GroupU32U32,
    u32,
    123456u32,
    u32,
    123456u32,
    GROUP_PAIR_U32_U32,
    GROUP_PAIR_U32_U32_INODES
);

group_pair_test!(
    value_match_group_pair_u32_u64,
    GroupU32U64,
    u32,
    123456u32,
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U32_U64,
    GROUP_PAIR_U32_U64_INODES
);

group_pair_test!(
    value_match_group_pair_u32_timestamp,
    GroupU32Timestamp,
    Time,
    Time(999_999_999),
    u32,
    123456u32,
    GROUP_PAIR_U32_TIMESTAMP,
    GROUP_PAIR_U32_TIMESTAMP_INODES
);

group_pair_test!(
    value_match_group_pair_u64_u64,
    GroupU64U64,
    u64,
    12345678901234567890u64,
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U64_U64,
    GROUP_PAIR_U64_U64_INODES
);

group_pair_test!(
    value_match_group_pair_u64_timestamp,
    GroupU64Timestamp,
    Time,
    Time(999_999_999),
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U64_TIMESTAMP,
    GROUP_PAIR_U64_TIMESTAMP_INODES
);

group_pair_test!(
    value_match_group_pair_timestamp_timestamp,
    GroupTimestampTimestamp,
    Time,
    Time(999_999_999),
    Time,
    Time(999_999_999),
    GROUP_PAIR_TIMESTAMP_TIMESTAMP,
    GROUP_PAIR_TIMESTAMP_TIMESTAMP_INODES
);

macro_rules! group_flat_test {
    (
        $test_name:ident,
        $group_ty:ident,
        { $($field:ident : $fty:ident = $fval:expr),+ $(,)? },
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
        struct $group_ty {
            $($field: $fty,)+
        }

        #[test]
        fn $test_name() {
            let data = $group_ty {
                    $($field: $fval,)+
                };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

group_flat_test!(
    value_match_group_five_mixed_order_a,
    GroupFiveMixedOrderA,
    {
        a: u64 = 12345678901234567890u64,
        b: bool = true,
        c: Time = Time(999_999_999),
        d: u16 = 54321u16,
        e: u32 = 123456u32,
    },
    GROUP_FIVE_MIXED_ORDER_A,
    GROUP_FIVE_MIXED_ORDER_A_INODES

);

group_flat_test!(
    value_match_group_five_mixed_order_b,
    GroupFiveMixedOrderB,
    {
        a: Time = Time(999_999_999),
        b: u32 = 123456u32,
        c: bool = true,
        d: u64 = 12345678901234567890u64,
        e: u16 = 54321u16,
    },
    GROUP_FIVE_MIXED_ORDER_B,
    GROUP_FIVE_MIXED_ORDER_B_INODES

);

group_flat_test!(
    value_match_group_signed_ints,
    GroupSignedInts,
    {
        a: i8 = -100i8,
        b: i16 = -12345i16,
        c: i32 = -123456i32,
        d: i64 = -1234567890123i64,
    },
    GROUP_SIGNED_INTS,
    GROUP_SIGNED_INTS_INODES

);

group_flat_test!(
    value_match_group_u8_floats,
    GroupU8Floats,
    {
        a: u8 = 200u8,
        b: f16 = f16::from_f32(1.5),
        c: f32 = 3.14f32,
        d: f64 = 2.718281828f64,
    },
    GROUP_U8_FLOATS,
    GROUP_U8_FLOATS_INODES

);

group_flat_test!(
    value_match_group_three_byte_aligned,
    GroupThreeByteAligned,
    {
        a: bool = true,
        b: i8 = -100i8,
        c: u8 = 200u8,
    },
    GROUP_THREE_BYTE_ALIGNED,
    GROUP_THREE_BYTE_ALIGNED_INODES

);

group_flat_test!(
    value_match_group_all_twelve_natural_order,
    GroupAllTwelveNaturalOrder,
    {
        f_bool: bool = true,
        f_i8: i8 = -100i8,
        f_i16: i16 = -12345i16,
        f_i32: i32 = -123456i32,
        f_i64: i64 = -1234567890123i64,
        f_u8: u8 = 200u8,
        f_u16: u16 = 54321u16,
        f_u32: u32 = 123456u32,
        f_u64: u64 = 12345678901234567890u64,
        f_f16: f16 = f16::from_f32(1.5),
        f_f32: f32 = 3.14f32,
        f_f64: f64 = 2.718281828f64,
    },
    GROUP_ALL_TWELVE_NATURAL_ORDER,
    GROUP_ALL_TWELVE_NATURAL_ORDER_INODES

);

group_flat_test!(
    value_match_group_all_twelve_reverse_order,
    GroupAllTwelveReverseOrder,
    {
        f_f64: f64 = 2.718281828f64,
        f_f32: f32 = 3.14f32,
        f_f16: f16 = f16::from_f32(1.5),
        f_u64: u64 = 12345678901234567890u64,
        f_u32: u32 = 123456u32,
        f_u16: u16 = 54321u16,
        f_u8: u8 = 200u8,
        f_i64: i64 = -1234567890123i64,
        f_i32: i32 = -123456i32,
        f_i16: i16 = -12345i16,
        f_i8: i8 = -100i8,
        f_bool: bool = true,
    },
    GROUP_ALL_TWELVE_REVERSE_ORDER,
    GROUP_ALL_TWELVE_REVERSE_ORDER_INODES

);

group_flat_test!(
    value_match_group_all_twelve_shuffled_order,
    GroupAllTwelveShuffledOrder,
    {
        f_u64: u64 = 12345678901234567890u64,
        f_bool: bool = true,
        f_f32: f32 = 3.14f32,
        f_i8: i8 = -100i8,
        f_f64: f64 = 2.718281828f64,
        f_u16: u16 = 54321u16,
        f_i32: i32 = -123456i32,
        f_f16: f16 = f16::from_f32(1.5),
        f_u8: u8 = 200u8,
        f_i64: i64 = -1234567890123i64,
        f_i16: i16 = -12345i16,
        f_u32: u32 = 123456u32,
    },
    GROUP_ALL_TWELVE_SHUFFLED_ORDER,
    GROUP_ALL_TWELVE_SHUFFLED_ORDER_INODES

);

group_flat_test!(
    value_match_group_eight_flat_fields,
    GroupEightFlatFields,
    {
        a: bool = true,
        b: i8 = -100i8,
        c: i16 = -12345i16,
        d: i32 = -123456i32,
        e: i64 = -1234567890123i64,
        f: u8 = 200u8,
        g: u16 = 54321u16,
        h: u32 = 123456u32,
    },
    GROUP_EIGHT_FLAT_FIELDS,
    GROUP_EIGHT_FLAT_FIELDS_INODES

);

group_flat_test!(
    value_match_group_single_field_timestamp,
    GroupSingleFieldTimestamp,
    {
        a: Time = Time(999_999_999),
    },
    GROUP_SINGLE_FIELD_TIMESTAMP,
    GROUP_SINGLE_FIELD_TIMESTAMP_INODES

);

group_flat_test!(
    value_match_group_single_field_bool,
    GroupSingleFieldBool,
    {
        a: bool = true,
    },
    GROUP_SINGLE_FIELD_BOOL,
    GROUP_SINGLE_FIELD_BOOL_INODES

);

group_flat_test!(
    value_match_group_four_identical_u8,
    GroupFourIdenticalU8,
    {
        a: u8 = 200u8,
        b: u8 = 200u8,
        c: u8 = 200u8,
        d: u8 = 200u8,
    },
    GROUP_FOUR_IDENTICAL_U8,
    GROUP_FOUR_IDENTICAL_U8_INODES

);

group_flat_test!(
    value_match_group_three_identical_u64,
    GroupThreeIdenticalU64,
    {
        a: u64 = 12345678901234567890u64,
        b: u64 = 12345678901234567890u64,
        c: u64 = 12345678901234567890u64,
    },
    GROUP_THREE_IDENTICAL_U64,
    GROUP_THREE_IDENTICAL_U64_INODES

);

group_flat_test!(
    value_match_group_two_identical_timestamp,
    GroupTwoIdenticalTimestamp,
    {
        a: Time = Time(999_999_999),
        b: Time = Time(999_999_999),
    },
    GROUP_TWO_IDENTICAL_TIMESTAMP,
    GROUP_TWO_IDENTICAL_TIMESTAMP_INODES

);

// ── Nested/sibling-group and multi-level-chain shapes ──────────────────
// Each needs its own chain of derive struct types (one per nesting level),
// so these are bespoke `#[test]` functions rather than macro instances.

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U8ThenNestedU64Inner {
    b: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U8ThenNestedU64Group {
    a: u8,
    inner: U8ThenNestedU64Inner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedU64ThenU8Inner {
    b: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedU64ThenU8Group {
    inner: NestedU64ThenU8Inner,
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TimestampThenNestedBoolInner {
    b: bool,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TimestampThenNestedBoolGroup {
    a: Time,
    inner: TimestampThenNestedBoolInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct BoolThenNestedTimestampInner {
    b: Time,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct BoolThenNestedTimestampGroup {
    a: bool,
    inner: BoolThenNestedTimestampInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedTwoThenU16Inner {
    b: u32,
    c: u8,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedTwoThenU16Group {
    inner: NestedTwoThenU16Inner,
    a: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U16ThenNestedTwoInner {
    b: u8,
    c: u32,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U16ThenNestedTwoGroup {
    a: u16,
    inner: U16ThenNestedTwoInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallInnerA {
    x: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallInnerB {
    y: u8,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallGroup {
    inner_a: TwoSiblingBigSmallInnerA,
    inner_b: TwoSiblingBigSmallInnerB,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigInnerA {
    x: u8,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigInnerB {
    y: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigGroup {
    inner_a: TwoSiblingSmallBigInnerA,
    inner_b: TwoSiblingSmallBigInnerB,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingInnerA {
    x: f64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingInnerB {
    y: bool,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingGroup {
    a: f32,
    inner_a: ScalarPlusTwoSiblingInnerA,
    inner_b: ScalarPlusTwoSiblingInnerB,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedInnerA {
    x: bool,
    y: u16,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedInnerB {
    z: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedGroup {
    inner_a: ThreeWayMixedInnerA,
    a: u8,
    inner_b: ThreeWayMixedInnerB,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedLevel2 {
    c: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedLevel1 {
    level2: TripleNestedLevel2,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedGroup {
    level1: TripleNestedLevel1,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelDeep {
    c: u32,
    d: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelMid {
    b: u16,
    deep: MixedThreeLevelDeep,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelGroup {
    a: u8,
    mid: MixedThreeLevelMid,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelL3 {
    x: Time,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelL2 {
    l3: FourLevelL3,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelL1 {
    l2: FourLevelL2,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelGroup {
    l1: FourLevelL1,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelL3 {
    v: u16,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelY {
    z: u8,
    l3: MixedFourLevelL3,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelA {
    x: bool,
    y: MixedFourLevelY,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelGroup {
    a: MixedFourLevelA,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL2a {
    l3a: u8,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL1a {
    l2a: TwoParallelChainsL2a,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL2b {
    l3b: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL1b {
    l2b: TwoParallelChainsL2b,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsGroup {
    l1a: TwoParallelChainsL1a,
    l1b: TwoParallelChainsL1b,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FiveFlatPlusNestedInner {
    x: bool,
    y: u8,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FiveFlatPlusNestedGroup {
    a: u64,
    b: f16,
    c: f32,
    d: f64,
    e: Time,
    inner: FiveFlatPlusNestedInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsInner1 {
    a: u8,
    b: u16,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsInner2 {
    c: u32,
    d: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsGroup {
    inner1: TwoNestedPairsInner1,
    inner2: TwoNestedPairsInner2,
    e: bool,
    f: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OneScalarPlusBigNestedInner {
    b: i16,
    c: i32,
    d: i64,
    e: u8,
    f: u16,
    g: u32,
    h: u64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OneScalarPlusBigNestedGroup {
    a: i8,
    inner: OneScalarPlusBigNestedInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AllTwelveWrappedInner {
    f_bool: bool,
    f_i8: i8,
    f_i16: i16,
    f_i32: i32,
    f_i64: i64,
    f_u8: u8,
    f_u16: u16,
    f_u32: u32,
    f_u64: u64,
    f_f16: f16,
    f_f32: f32,
    f_f64: f64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AllTwelveWrappedGroup {
    inner: AllTwelveWrappedInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedSingleFieldBoolInner {
    a: bool,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedSingleFieldBoolGroup {
    inner: NestedSingleFieldBoolInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedFourIdenticalU32Inner {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedFourIdenticalU32Group {
    inner: NestedFourIdenticalU32Inner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsInner1 {
    f_bool: bool,
    f_i8: i8,
    f_i16: i16,
    f_i32: i32,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsInner2 {
    f_i64: i64,
    f_u8: u8,
    f_u16: u16,
    f_u32: u32,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsInner3 {
    f_u64: u64,
    f_f16: f16,
    f_f32: f32,
    f_f64: f64,
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsGroup {
    inner_1: SplitThreeSiblingsInner1,
    inner_2: SplitThreeSiblingsInner2,
    inner_3: SplitThreeSiblingsInner3,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct EmptyGroup {}

// The empty group's own inode is what this pins, so it stays wrapped: a
// schema with no fields at all has no inodes.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct EmptyOuter {
    a: EmptyGroup,
}

#[test]
fn value_match_group_u8_then_nested_u64() {
    let data = U8ThenNestedU64Group {
        a: 200u8,
        inner: U8ThenNestedU64Inner {
            b: 12345678901234567890u64,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_U8_THEN_NESTED_U64,
        &GROUP_U8_THEN_NESTED_U64_INODES,
    );
}

#[test]
fn value_match_group_nested_u64_then_u8() {
    let data = NestedU64ThenU8Group {
        inner: NestedU64ThenU8Inner {
            b: 12345678901234567890u64,
        },
        a: 200u8,
    };
    assert_roundtrip(
        &data,
        &GROUP_NESTED_U64_THEN_U8,
        &GROUP_NESTED_U64_THEN_U8_INODES,
    );
}

#[test]
fn value_match_group_timestamp_then_nested_bool() {
    let data = TimestampThenNestedBoolGroup {
        a: Time(999_999_999),
        inner: TimestampThenNestedBoolInner { b: true },
    };
    assert_roundtrip(
        &data,
        &GROUP_TIMESTAMP_THEN_NESTED_BOOL,
        &GROUP_TIMESTAMP_THEN_NESTED_BOOL_INODES,
    );
}

#[test]
fn value_match_group_bool_then_nested_timestamp() {
    let data = BoolThenNestedTimestampGroup {
        a: true,
        inner: BoolThenNestedTimestampInner {
            b: Time(999_999_999),
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_BOOL_THEN_NESTED_TIMESTAMP,
        &GROUP_BOOL_THEN_NESTED_TIMESTAMP_INODES,
    );
}

#[test]
fn value_match_group_nested_two_then_u16() {
    let data = NestedTwoThenU16Group {
        inner: NestedTwoThenU16Inner {
            b: 123456u32,
            c: 200u8,
        },
        a: 54321u16,
    };
    assert_roundtrip(
        &data,
        &GROUP_NESTED_TWO_THEN_U16,
        &GROUP_NESTED_TWO_THEN_U16_INODES,
    );
}

#[test]
fn value_match_group_u16_then_nested_two() {
    let data = U16ThenNestedTwoGroup {
        a: 54321u16,
        inner: U16ThenNestedTwoInner {
            b: 200u8,
            c: 123456u32,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_U16_THEN_NESTED_TWO,
        &GROUP_U16_THEN_NESTED_TWO_INODES,
    );
}

#[test]
fn value_match_group_two_sibling_nested_groups_big_small() {
    let data = TwoSiblingBigSmallGroup {
        inner_a: TwoSiblingBigSmallInnerA {
            x: 12345678901234567890u64,
        },
        inner_b: TwoSiblingBigSmallInnerB { y: 200u8 },
    };
    assert_roundtrip(
        &data,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_BIG_SMALL,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_BIG_SMALL_INODES,
    );
}

#[test]
fn value_match_group_two_sibling_nested_groups_small_big() {
    let data = TwoSiblingSmallBigGroup {
        inner_a: TwoSiblingSmallBigInnerA { x: 200u8 },
        inner_b: TwoSiblingSmallBigInnerB {
            y: 12345678901234567890u64,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_SMALL_BIG,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_SMALL_BIG_INODES,
    );
}

#[test]
fn value_match_group_scalar_plus_two_sibling_groups() {
    let data = ScalarPlusTwoSiblingGroup {
        a: 3.14f32,
        inner_a: ScalarPlusTwoSiblingInnerA { x: 2.718281828f64 },
        inner_b: ScalarPlusTwoSiblingInnerB { y: true },
    };
    assert_roundtrip(
        &data,
        &GROUP_SCALAR_PLUS_TWO_SIBLING_GROUPS,
        &GROUP_SCALAR_PLUS_TWO_SIBLING_GROUPS_INODES,
    );
}

#[test]
fn value_match_group_three_way_mixed_nesting() {
    let data = ThreeWayMixedGroup {
        inner_a: ThreeWayMixedInnerA {
            x: true,
            y: 54321u16,
        },
        a: 200u8,
        inner_b: ThreeWayMixedInnerB {
            z: 12345678901234567890u64,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_THREE_WAY_MIXED_NESTING,
        &GROUP_THREE_WAY_MIXED_NESTING_INODES,
    );
}

#[test]
fn value_match_group_triple_nested_single_leaf() {
    let data = TripleNestedGroup {
        level1: TripleNestedLevel1 {
            level2: TripleNestedLevel2 {
                c: 12345678901234567890u64,
            },
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_TRIPLE_NESTED_SINGLE_LEAF,
        &GROUP_TRIPLE_NESTED_SINGLE_LEAF_INODES,
    );
}

#[test]
fn value_match_group_mixed_three_level() {
    let data = MixedThreeLevelGroup {
        a: 200u8,
        mid: MixedThreeLevelMid {
            b: 54321u16,
            deep: MixedThreeLevelDeep {
                c: 123456u32,
                d: 12345678901234567890u64,
            },
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_MIXED_THREE_LEVEL,
        &GROUP_MIXED_THREE_LEVEL_INODES,
    );
}

#[test]
fn value_match_group_four_level_single_leaf() {
    let data = FourLevelGroup {
        l1: FourLevelL1 {
            l2: FourLevelL2 {
                l3: FourLevelL3 {
                    x: Time(999_999_999),
                },
            },
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_FOUR_LEVEL_SINGLE_LEAF,
        &GROUP_FOUR_LEVEL_SINGLE_LEAF_INODES,
    );
}

#[test]
fn value_match_group_mixed_four_level() {
    let data = MixedFourLevelGroup {
        a: MixedFourLevelA {
            x: true,
            y: MixedFourLevelY {
                z: 200u8,
                l3: MixedFourLevelL3 { v: 54321u16 },
            },
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_MIXED_FOUR_LEVEL,
        &GROUP_MIXED_FOUR_LEVEL_INODES,
    );
}

#[test]
fn value_match_group_two_parallel_deep_chains() {
    let data = TwoParallelChainsGroup {
        l1a: TwoParallelChainsL1a {
            l2a: TwoParallelChainsL2a { l3a: 200u8 },
        },
        l1b: TwoParallelChainsL1b {
            l2b: TwoParallelChainsL2b {
                l3b: 12345678901234567890u64,
            },
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_TWO_PARALLEL_DEEP_CHAINS,
        &GROUP_TWO_PARALLEL_DEEP_CHAINS_INODES,
    );
}

#[test]
fn value_match_group_five_flat_plus_nested_pair() {
    let data = FiveFlatPlusNestedGroup {
        a: 12345678901234567890u64,
        b: f16::from_f32(1.5),
        c: 3.14f32,
        d: 2.718281828f64,
        e: Time(999_999_999),
        inner: FiveFlatPlusNestedInner { x: true, y: 200u8 },
    };
    assert_roundtrip(
        &data,
        &GROUP_FIVE_FLAT_PLUS_NESTED_PAIR,
        &GROUP_FIVE_FLAT_PLUS_NESTED_PAIR_INODES,
    );
}

#[test]
fn value_match_group_two_nested_pairs_plus_two_scalars() {
    let data = TwoNestedPairsGroup {
        inner1: TwoNestedPairsInner1 {
            a: 200u8,
            b: 54321u16,
        },
        inner2: TwoNestedPairsInner2 {
            c: 123456u32,
            d: 12345678901234567890u64,
        },
        e: true,
        f: Time(999_999_999),
    };
    assert_roundtrip(
        &data,
        &GROUP_TWO_NESTED_PAIRS_PLUS_TWO_SCALARS,
        &GROUP_TWO_NESTED_PAIRS_PLUS_TWO_SCALARS_INODES,
    );
}

#[test]
fn value_match_group_one_scalar_plus_big_nested() {
    let data = OneScalarPlusBigNestedGroup {
        a: -100i8,
        inner: OneScalarPlusBigNestedInner {
            b: -12345i16,
            c: -123456i32,
            d: -1234567890123i64,
            e: 200u8,
            f: 54321u16,
            g: 123456u32,
            h: 12345678901234567890u64,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_ONE_SCALAR_PLUS_BIG_NESTED,
        &GROUP_ONE_SCALAR_PLUS_BIG_NESTED_INODES,
    );
}

#[test]
fn value_match_group_all_twelve_wrapped_in_nested_group() {
    let data = AllTwelveWrappedGroup {
        inner: AllTwelveWrappedInner {
            f_bool: true,
            f_i8: -100i8,
            f_i16: -12345i16,
            f_i32: -123456i32,
            f_i64: -1234567890123i64,
            f_u8: 200u8,
            f_u16: 54321u16,
            f_u32: 123456u32,
            f_u64: 12345678901234567890u64,
            f_f16: f16::from_f32(1.5),
            f_f32: 3.14f32,
            f_f64: 2.718281828f64,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_ALL_TWELVE_WRAPPED_IN_NESTED_GROUP,
        &GROUP_ALL_TWELVE_WRAPPED_IN_NESTED_GROUP_INODES,
    );
}

#[test]
fn value_match_group_nested_single_field_bool() {
    let data = NestedSingleFieldBoolGroup {
        inner: NestedSingleFieldBoolInner { a: true },
    };
    assert_roundtrip(
        &data,
        &GROUP_NESTED_SINGLE_FIELD_BOOL,
        &GROUP_NESTED_SINGLE_FIELD_BOOL_INODES,
    );
}

#[test]
fn value_match_group_nested_four_identical_u32() {
    let data = NestedFourIdenticalU32Group {
        inner: NestedFourIdenticalU32Inner {
            a: 123456u32,
            b: 123456u32,
            c: 123456u32,
            d: 123456u32,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_NESTED_FOUR_IDENTICAL_U32,
        &GROUP_NESTED_FOUR_IDENTICAL_U32_INODES,
    );
}

#[test]
fn value_match_group_all_twelve_split_across_three_siblings_matches_single_group() {
    let data = SplitThreeSiblingsGroup {
        inner_1: SplitThreeSiblingsInner1 {
            f_bool: true,
            f_i8: -100i8,
            f_i16: -12345i16,
            f_i32: -123456i32,
        },
        inner_2: SplitThreeSiblingsInner2 {
            f_i64: -1234567890123i64,
            f_u8: 200u8,
            f_u16: 54321u16,
            f_u32: 123456u32,
        },
        inner_3: SplitThreeSiblingsInner3 {
            f_u64: 12345678901234567890u64,
            f_f16: f16::from_f32(1.5),
            f_f32: 3.14f32,
            f_f64: 2.718281828f64,
        },
    };
    assert_roundtrip(
        &data,
        &GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP,
        &GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP_INODES,
    );
}

#[test]
fn value_match_group_empty() {
    let data = EmptyOuter { a: EmptyGroup {} };
    assert_roundtrip(&data, &GROUP_EMPTY_BUF, &GROUP_EMPTY_INODES);
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsLadderAscending {
    a: u8,
    #[pyro(numeric)]
    b: Vec<u8>,
    c: Vec<u16>,
    d: Vec<u32>,
    e: Vec<u64>,
    f: Vec<Time>,
    g: String,
}

#[test]
fn value_match_group_gaps_ladder_ascending() {
    let data = GroupGapsLadderAscending {
        a: 7,
        b: vec![1, 2, 3],
        c: vec![4],
        d: vec![5],
        e: vec![6],
        f: vec![Time(1000000007)],
        g: "xyz".to_string(),
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_LADDER_ASCENDING,
        &GROUP_GAPS_LADDER_ASCENDING_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsLadderDescending {
    a: Vec<Time>,
    b: Vec<u64>,
    c: Vec<u32>,
    d: Vec<u16>,
    #[pyro(numeric)]
    e: Vec<u8>,
    f: bool,
    g: Vec<u8>,
}

#[test]
fn value_match_group_gaps_ladder_descending() {
    let data = GroupGapsLadderDescending {
        a: vec![Time(1000000007), Time(-1000000007)],
        b: vec![1],
        c: vec![2, 3, 4],
        d: vec![5],
        e: vec![6, 7, 8, 9, 10],
        f: true,
        g: vec![9, 8],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_LADDER_DESCENDING,
        &GROUP_GAPS_LADDER_DESCENDING_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsOddLengths {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
    #[pyro(numeric)]
    b: Vec<u8>,
    c: Vec<Vec<u64>>,
    d: Vec<[u8; 3]>,
    e: Vec<[u32; 3]>,
    f: Vec<u8>,
    g: Vec<Vec<Vec<u16>>>,
    h: Vec<i64>,
}

#[test]
fn value_match_group_gaps_odd_lengths() {
    let data = GroupGapsOddLengths {
        a: vec![true, false, true, true, false],
        b: vec![1],
        c: vec![vec![1], vec![2, 3]],
        d: vec![[1, 2, 3], [4, 5, 6]],
        e: vec![[7, 8, 9]],
        f: vec![1, 2, 3, 4, 5],
        g: vec![vec![vec![1], vec![2, 3]], vec![vec![4]]],
        h: vec![-1],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_ODD_LENGTHS,
        &GROUP_GAPS_ODD_LENGTHS_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsNestedGroupsA {
    x: Vec<u16>,
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsNestedGroupsB {
    p: u64,
    #[pyro(numeric)]
    q: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsNestedGroupsC {
    r: Vec<Time>,
    s: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsNestedGroups {
    a: GroupGapsNestedGroupsA,
    b: Vec<GroupGapsNestedGroupsB>,
    c: GroupGapsNestedGroupsC,
    d: String,
}

#[test]
fn value_match_group_gaps_nested_groups() {
    let data = GroupGapsNestedGroups {
        a: GroupGapsNestedGroupsA {
            x: vec![1, 2, 3],
            y: 9,
        },
        b: vec![
            GroupGapsNestedGroupsB { p: 1, q: vec![1] },
            GroupGapsNestedGroupsB {
                p: 2,
                q: vec![2, 3],
            },
        ],
        c: GroupGapsNestedGroupsC {
            r: vec![Time(5)],
            s: true,
        },
        d: "odd".to_string(),
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_NESTED_GROUPS,
        &GROUP_GAPS_NESTED_GROUPS_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsEmptyBetween {
    a: Vec<u64>,
    #[pyro(numeric)]
    b: Vec<u8>,
    c: Vec<Time>,
    d: Vec<u32>,
    e: String,
    f: Vec<Vec<f64>>,
    g: Vec<f16>,
}

#[test]
fn value_match_group_gaps_empty_between() {
    let data = GroupGapsEmptyBetween {
        a: vec![],
        b: vec![1, 2, 3],
        c: vec![],
        d: vec![1],
        e: "".to_string(),
        f: vec![vec![], vec![2.5]],
        g: vec![f16::from_f32(1.5)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_EMPTY_BETWEEN,
        &GROUP_GAPS_EMPTY_BETWEEN_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsFixedArrays {
    a: [u8; 3],
    b: Vec<[u16; 3]>,
    c: [u64; 1],
    d: Vec<[bool; 3]>,
    e: Vec<[f32; 2]>,
    f: [i16; 5],
}

#[test]
fn value_match_group_gaps_fixed_arrays() {
    let data = GroupGapsFixedArrays {
        a: [1, 2, 3],
        b: vec![[1, 2, 3]],
        c: [7],
        d: vec![[true, false, true], [false, false, true]],
        e: vec![[1.5, 2.5]],
        f: [-1, -2, -3, -4, -5],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_FIXED_ARRAYS,
        &GROUP_GAPS_FIXED_ARRAYS_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfter1InlineBytes {
    a: [u8; 1],
    b: Vec<Time>,
}

#[test]
fn value_match_group_gaps_timestamp_after_1_inline_bytes() {
    let data = GroupGapsTimestampAfter1InlineBytes {
        a: [1],
        b: vec![Time(1000000007)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_1_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_1_INLINE_BYTES_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfter3InlineBytes {
    a: [u8; 3],
    b: Vec<Time>,
}

#[test]
fn value_match_group_gaps_timestamp_after_3_inline_bytes() {
    let data = GroupGapsTimestampAfter3InlineBytes {
        a: [1, 2, 3],
        b: vec![Time(1000000007)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_3_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_3_INLINE_BYTES_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfter6InlineBytes {
    a: [u8; 6],
    b: Vec<Time>,
}

#[test]
fn value_match_group_gaps_timestamp_after_6_inline_bytes() {
    let data = GroupGapsTimestampAfter6InlineBytes {
        a: [1, 2, 3, 4, 5, 6],
        b: vec![Time(1000000007)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_6_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_6_INLINE_BYTES_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfter9InlineBytes {
    a: [u8; 9],
    b: Vec<Time>,
}

#[test]
fn value_match_group_gaps_timestamp_after_9_inline_bytes() {
    let data = GroupGapsTimestampAfter9InlineBytes {
        a: [1, 2, 3, 4, 5, 6, 7, 8, 9],
        b: vec![Time(1000000007)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_9_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_9_INLINE_BYTES_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfter12InlineBytes {
    a: [u8; 12],
    b: Vec<Time>,
}

#[test]
fn value_match_group_gaps_timestamp_after_12_inline_bytes() {
    let data = GroupGapsTimestampAfter12InlineBytes {
        a: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        b: vec![Time(1000000007)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_12_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_12_INLINE_BYTES_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfter15InlineBytes {
    a: [u8; 15],
    b: Vec<Time>,
}

#[test]
fn value_match_group_gaps_timestamp_after_15_inline_bytes() {
    let data = GroupGapsTimestampAfter15InlineBytes {
        a: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        b: vec![Time(1000000007)],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_15_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_15_INLINE_BYTES_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupGapsTimestampAfterTopology {
    a: [u8; 5],
    b: Vec<Vec<Time>>,
}

#[test]
fn value_match_group_gaps_timestamp_after_topology() {
    let data = GroupGapsTimestampAfterTopology {
        a: [1, 2, 3, 4, 5],
        b: vec![
            vec![Time(1000000007)],
            vec![],
            vec![Time(1000000007), Time(-1000000007)],
        ],
    };
    assert_roundtrip(
        &data,
        &GROUP_GAPS_TIMESTAMP_AFTER_TOPOLOGY,
        &GROUP_GAPS_TIMESTAMP_AFTER_TOPOLOGY_INODES,
    );
}
