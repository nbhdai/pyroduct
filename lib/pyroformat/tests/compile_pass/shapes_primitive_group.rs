//! Every derived struct shape from `tests/value_match/primitive_group_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

macro_rules! group_pair_test {
    (
        $word:ident,
        $test_name:ident,
        $group_ty:ident,
        $outer_ty:ident,
        $x_ty:ident,
        $x_val:expr,
        $y_ty:ident,
        $y_val:expr,
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $group_ty {
            x: $x_ty,
            y: $y_ty,
        }

        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $outer_ty {
            a: $group_ty,
        }

        const _: fn() = assert_pyro_traits::<$outer_ty>;
    };
}

group_pair_test!(
    u32,
    value_match_group_pair_bool_bool,
    GroupBoolBool32,
    OuterBoolBool32,
    bool,
    true,
    bool,
    true,
    GROUP_PAIR_BOOL_BOOL,
    GROUP_PAIR_BOOL_BOOL_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_bool_bool,
    GroupBoolBool16,
    OuterBoolBool16,
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
    u32,
    value_match_group_pair_bool_u16,
    GroupBoolU16W32,
    OuterBoolU16W32,
    u16,
    54321u16,
    bool,
    true,
    GROUP_PAIR_BOOL_U16,
    GROUP_PAIR_BOOL_U16_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_bool_u16,
    GroupBoolU16W16,
    OuterBoolU16W16,
    u16,
    54321u16,
    bool,
    true,
    GROUP_PAIR_BOOL_U16,
    GROUP_PAIR_BOOL_U16_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_bool_u32,
    GroupBoolU32W32,
    OuterBoolU32W32,
    bool,
    true,
    u32,
    123456u32,
    GROUP_PAIR_BOOL_U32,
    GROUP_PAIR_BOOL_U32_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_bool_u32,
    GroupBoolU32W16,
    OuterBoolU32W16,
    bool,
    true,
    u32,
    123456u32,
    GROUP_PAIR_BOOL_U32,
    GROUP_PAIR_BOOL_U32_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_bool_u64,
    GroupBoolU64W32,
    OuterBoolU64W32,
    u64,
    12345678901234567890u64,
    bool,
    true,
    GROUP_PAIR_BOOL_U64,
    GROUP_PAIR_BOOL_U64_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_bool_u64,
    GroupBoolU64W16,
    OuterBoolU64W16,
    u64,
    12345678901234567890u64,
    bool,
    true,
    GROUP_PAIR_BOOL_U64,
    GROUP_PAIR_BOOL_U64_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_bool_timestamp,
    GroupBoolTimestamp32,
    OuterBoolTimestamp32,
    bool,
    true,
    Time,
    Time(999_999_999),
    GROUP_PAIR_BOOL_TIMESTAMP,
    GROUP_PAIR_BOOL_TIMESTAMP_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_bool_timestamp,
    GroupBoolTimestamp16,
    OuterBoolTimestamp16,
    bool,
    true,
    Time,
    Time(999_999_999),
    GROUP_PAIR_BOOL_TIMESTAMP,
    GROUP_PAIR_BOOL_TIMESTAMP_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u16_u16,
    GroupU16U16W32,
    OuterU16U16W32,
    u16,
    54321u16,
    u16,
    54321u16,
    GROUP_PAIR_U16_U16,
    GROUP_PAIR_U16_U16_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u16_u16,
    GroupU16U16W16,
    OuterU16U16W16,
    u16,
    54321u16,
    u16,
    54321u16,
    GROUP_PAIR_U16_U16,
    GROUP_PAIR_U16_U16_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u16_u32,
    GroupU16U32W32,
    OuterU16U32W32,
    u16,
    54321u16,
    u32,
    123456u32,
    GROUP_PAIR_U16_U32,
    GROUP_PAIR_U16_U32_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u16_u32,
    GroupU16U32W16,
    OuterU16U32W16,
    u16,
    54321u16,
    u32,
    123456u32,
    GROUP_PAIR_U16_U32,
    GROUP_PAIR_U16_U32_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u16_u64,
    GroupU16U64W32,
    OuterU16U64W32,
    u64,
    12345678901234567890u64,
    u16,
    54321u16,
    GROUP_PAIR_U16_U64,
    GROUP_PAIR_U16_U64_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u16_u64,
    GroupU16U64W16,
    OuterU16U64W16,
    u64,
    12345678901234567890u64,
    u16,
    54321u16,
    GROUP_PAIR_U16_U64,
    GROUP_PAIR_U16_U64_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u16_timestamp,
    GroupU16Timestamp32,
    OuterU16Timestamp32,
    u16,
    54321u16,
    Time,
    Time(999_999_999),
    GROUP_PAIR_U16_TIMESTAMP,
    GROUP_PAIR_U16_TIMESTAMP_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u16_timestamp,
    GroupU16Timestamp16,
    OuterU16Timestamp16,
    u16,
    54321u16,
    Time,
    Time(999_999_999),
    GROUP_PAIR_U16_TIMESTAMP,
    GROUP_PAIR_U16_TIMESTAMP_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u32_u32,
    GroupU32U32W32,
    OuterU32U32W32,
    u32,
    123456u32,
    u32,
    123456u32,
    GROUP_PAIR_U32_U32,
    GROUP_PAIR_U32_U32_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u32_u32,
    GroupU32U32W16,
    OuterU32U32W16,
    u32,
    123456u32,
    u32,
    123456u32,
    GROUP_PAIR_U32_U32,
    GROUP_PAIR_U32_U32_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u32_u64,
    GroupU32U64W32,
    OuterU32U64W32,
    u32,
    123456u32,
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U32_U64,
    GROUP_PAIR_U32_U64_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u32_u64,
    GroupU32U64W16,
    OuterU32U64W16,
    u32,
    123456u32,
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U32_U64,
    GROUP_PAIR_U32_U64_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u32_timestamp,
    GroupU32Timestamp32,
    OuterU32Timestamp32,
    Time,
    Time(999_999_999),
    u32,
    123456u32,
    GROUP_PAIR_U32_TIMESTAMP,
    GROUP_PAIR_U32_TIMESTAMP_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u32_timestamp,
    GroupU32Timestamp16,
    OuterU32Timestamp16,
    Time,
    Time(999_999_999),
    u32,
    123456u32,
    GROUP_PAIR_U32_TIMESTAMP,
    GROUP_PAIR_U32_TIMESTAMP_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u64_u64,
    GroupU64U64W32,
    OuterU64U64W32,
    u64,
    12345678901234567890u64,
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U64_U64,
    GROUP_PAIR_U64_U64_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u64_u64,
    GroupU64U64W16,
    OuterU64U64W16,
    u64,
    12345678901234567890u64,
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U64_U64,
    GROUP_PAIR_U64_U64_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_u64_timestamp,
    GroupU64Timestamp32,
    OuterU64Timestamp32,
    Time,
    Time(999_999_999),
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U64_TIMESTAMP,
    GROUP_PAIR_U64_TIMESTAMP_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_u64_timestamp,
    GroupU64Timestamp16,
    OuterU64Timestamp16,
    Time,
    Time(999_999_999),
    u64,
    12345678901234567890u64,
    GROUP_PAIR_U64_TIMESTAMP,
    GROUP_PAIR_U64_TIMESTAMP_INODES
);

group_pair_test!(
    u32,
    value_match_group_pair_timestamp_timestamp,
    GroupTimestampTimestamp32,
    OuterTimestampTimestamp32,
    Time,
    Time(999_999_999),
    Time,
    Time(999_999_999),
    GROUP_PAIR_TIMESTAMP_TIMESTAMP,
    GROUP_PAIR_TIMESTAMP_TIMESTAMP_INODES
);

group_pair_test!(
    u16,
    value_match_group_pair_timestamp_timestamp,
    GroupTimestampTimestamp16,
    OuterTimestampTimestamp16,
    Time,
    Time(999_999_999),
    Time,
    Time(999_999_999),
    GROUP_PAIR_TIMESTAMP_TIMESTAMP,
    GROUP_PAIR_TIMESTAMP_TIMESTAMP_INODES
);

macro_rules! group_flat_test {
    (
        $word:ident,
        $test_name:ident,
        $group_ty:ident,
        $outer_ty:ident,
        { $($field:ident : $fty:ident = $fval:expr),+ $(,)? },
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $group_ty {
            $($field: $fty,)+
        }

        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $outer_ty {
            a: $group_ty,
        }

        const _: fn() = assert_pyro_traits::<$outer_ty>;
    };
}

group_flat_test!(
    u32,
    value_match_group_five_mixed_order_a,
    GroupFiveMixedOrderA32,
    OuterFiveMixedOrderA32,
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
    u16,
    value_match_group_five_mixed_order_a,
    GroupFiveMixedOrderA16,
    OuterFiveMixedOrderA16,
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
    u32,
    value_match_group_five_mixed_order_b,
    GroupFiveMixedOrderB32,
    OuterFiveMixedOrderB32,
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
    u16,
    value_match_group_five_mixed_order_b,
    GroupFiveMixedOrderB16,
    OuterFiveMixedOrderB16,
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
    u32,
    value_match_group_signed_ints,
    GroupSignedInts32,
    OuterSignedInts32,
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
    u16,
    value_match_group_signed_ints,
    GroupSignedInts16,
    OuterSignedInts16,
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
    u32,
    value_match_group_u8_floats,
    GroupU8Floats32,
    OuterU8Floats32,
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
    u16,
    value_match_group_u8_floats,
    GroupU8Floats16,
    OuterU8Floats16,
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
    u32,
    value_match_group_three_byte_aligned,
    GroupThreeByteAligned32,
    OuterThreeByteAligned32,
    {
        a: bool = true,
        b: i8 = -100i8,
        c: u8 = 200u8,
    },
    GROUP_THREE_BYTE_ALIGNED,
    GROUP_THREE_BYTE_ALIGNED_INODES
);

group_flat_test!(
    u16,
    value_match_group_three_byte_aligned,
    GroupThreeByteAligned16,
    OuterThreeByteAligned16,
    {
        a: bool = true,
        b: i8 = -100i8,
        c: u8 = 200u8,
    },
    GROUP_THREE_BYTE_ALIGNED,
    GROUP_THREE_BYTE_ALIGNED_INODES
);

group_flat_test!(
    u32,
    value_match_group_all_twelve_natural_order,
    GroupAllTwelveNaturalOrder32,
    OuterAllTwelveNaturalOrder32,
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
    u16,
    value_match_group_all_twelve_natural_order,
    GroupAllTwelveNaturalOrder16,
    OuterAllTwelveNaturalOrder16,
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
    u32,
    value_match_group_all_twelve_reverse_order,
    GroupAllTwelveReverseOrder32,
    OuterAllTwelveReverseOrder32,
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
    u16,
    value_match_group_all_twelve_reverse_order,
    GroupAllTwelveReverseOrder16,
    OuterAllTwelveReverseOrder16,
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
    u32,
    value_match_group_all_twelve_shuffled_order,
    GroupAllTwelveShuffledOrder32,
    OuterAllTwelveShuffledOrder32,
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
    u16,
    value_match_group_all_twelve_shuffled_order,
    GroupAllTwelveShuffledOrder16,
    OuterAllTwelveShuffledOrder16,
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
    u32,
    value_match_group_eight_flat_fields,
    GroupEightFlatFields32,
    OuterEightFlatFields32,
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
    u16,
    value_match_group_eight_flat_fields,
    GroupEightFlatFields16,
    OuterEightFlatFields16,
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
    u32,
    value_match_group_single_field_timestamp,
    GroupSingleFieldTimestamp32,
    OuterSingleFieldTimestamp32,
    {
        a: Time = Time(999_999_999),
    },
    GROUP_SINGLE_FIELD_TIMESTAMP,
    GROUP_SINGLE_FIELD_TIMESTAMP_INODES
);

group_flat_test!(
    u16,
    value_match_group_single_field_timestamp,
    GroupSingleFieldTimestamp16,
    OuterSingleFieldTimestamp16,
    {
        a: Time = Time(999_999_999),
    },
    GROUP_SINGLE_FIELD_TIMESTAMP,
    GROUP_SINGLE_FIELD_TIMESTAMP_INODES
);

group_flat_test!(
    u32,
    value_match_group_single_field_bool,
    GroupSingleFieldBool32,
    OuterSingleFieldBool32,
    {
        a: bool = true,
    },
    GROUP_SINGLE_FIELD_BOOL,
    GROUP_SINGLE_FIELD_BOOL_INODES
);

group_flat_test!(
    u16,
    value_match_group_single_field_bool,
    GroupSingleFieldBool16,
    OuterSingleFieldBool16,
    {
        a: bool = true,
    },
    GROUP_SINGLE_FIELD_BOOL,
    GROUP_SINGLE_FIELD_BOOL_INODES
);

group_flat_test!(
    u32,
    value_match_group_four_identical_u8,
    GroupFourIdenticalU8W32,
    OuterFourIdenticalU8W32,
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
    u16,
    value_match_group_four_identical_u8,
    GroupFourIdenticalU8W16,
    OuterFourIdenticalU8W16,
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
    u32,
    value_match_group_three_identical_u64,
    GroupThreeIdenticalU64W32,
    OuterThreeIdenticalU64W32,
    {
        a: u64 = 12345678901234567890u64,
        b: u64 = 12345678901234567890u64,
        c: u64 = 12345678901234567890u64,
    },
    GROUP_THREE_IDENTICAL_U64,
    GROUP_THREE_IDENTICAL_U64_INODES
);

group_flat_test!(
    u16,
    value_match_group_three_identical_u64,
    GroupThreeIdenticalU64W16,
    OuterThreeIdenticalU64W16,
    {
        a: u64 = 12345678901234567890u64,
        b: u64 = 12345678901234567890u64,
        c: u64 = 12345678901234567890u64,
    },
    GROUP_THREE_IDENTICAL_U64,
    GROUP_THREE_IDENTICAL_U64_INODES
);

group_flat_test!(
    u32,
    value_match_group_two_identical_timestamp,
    GroupTwoIdenticalTimestamp32,
    OuterTwoIdenticalTimestamp32,
    {
        a: Time = Time(999_999_999),
        b: Time = Time(999_999_999),
    },
    GROUP_TWO_IDENTICAL_TIMESTAMP,
    GROUP_TWO_IDENTICAL_TIMESTAMP_INODES
);

group_flat_test!(
    u16,
    value_match_group_two_identical_timestamp,
    GroupTwoIdenticalTimestamp16,
    OuterTwoIdenticalTimestamp16,
    {
        a: Time = Time(999_999_999),
        b: Time = Time(999_999_999),
    },
    GROUP_TWO_IDENTICAL_TIMESTAMP,
    GROUP_TWO_IDENTICAL_TIMESTAMP_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U8ThenNestedU64Inner32 {
    b: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct U8ThenNestedU64Inner16 {
    b: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U8ThenNestedU64Group32 {
    a: u8,
    inner: U8ThenNestedU64Inner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct U8ThenNestedU64Group16 {
    a: u8,
    inner: U8ThenNestedU64Inner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U8ThenNestedU64Outer32 {
    a: U8ThenNestedU64Group32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct U8ThenNestedU64Outer16 {
    a: U8ThenNestedU64Group16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedU64ThenU8Inner32 {
    b: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedU64ThenU8Inner16 {
    b: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedU64ThenU8Group32 {
    inner: NestedU64ThenU8Inner32,
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedU64ThenU8Group16 {
    inner: NestedU64ThenU8Inner16,
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedU64ThenU8Outer32 {
    a: NestedU64ThenU8Group32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedU64ThenU8Outer16 {
    a: NestedU64ThenU8Group16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TimestampThenNestedBoolInner32 {
    b: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TimestampThenNestedBoolInner16 {
    b: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TimestampThenNestedBoolGroup32 {
    a: Time,
    inner: TimestampThenNestedBoolInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TimestampThenNestedBoolGroup16 {
    a: Time,
    inner: TimestampThenNestedBoolInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TimestampThenNestedBoolOuter32 {
    a: TimestampThenNestedBoolGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TimestampThenNestedBoolOuter16 {
    a: TimestampThenNestedBoolGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct BoolThenNestedTimestampInner32 {
    b: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct BoolThenNestedTimestampInner16 {
    b: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct BoolThenNestedTimestampGroup32 {
    a: bool,
    inner: BoolThenNestedTimestampInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct BoolThenNestedTimestampGroup16 {
    a: bool,
    inner: BoolThenNestedTimestampInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct BoolThenNestedTimestampOuter32 {
    a: BoolThenNestedTimestampGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct BoolThenNestedTimestampOuter16 {
    a: BoolThenNestedTimestampGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedTwoThenU16Inner32 {
    b: u32,
    c: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedTwoThenU16Inner16 {
    b: u32,
    c: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedTwoThenU16Group32 {
    inner: NestedTwoThenU16Inner32,
    a: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedTwoThenU16Group16 {
    inner: NestedTwoThenU16Inner16,
    a: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedTwoThenU16Outer32 {
    a: NestedTwoThenU16Group32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedTwoThenU16Outer16 {
    a: NestedTwoThenU16Group16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U16ThenNestedTwoInner32 {
    b: u8,
    c: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct U16ThenNestedTwoInner16 {
    b: u8,
    c: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U16ThenNestedTwoGroup32 {
    a: u16,
    inner: U16ThenNestedTwoInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct U16ThenNestedTwoGroup16 {
    a: u16,
    inner: U16ThenNestedTwoInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct U16ThenNestedTwoOuter32 {
    a: U16ThenNestedTwoGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct U16ThenNestedTwoOuter16 {
    a: U16ThenNestedTwoGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallInnerA32 {
    x: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingBigSmallInnerA16 {
    x: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallInnerB32 {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingBigSmallInnerB16 {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallGroup32 {
    inner_a: TwoSiblingBigSmallInnerA32,
    inner_b: TwoSiblingBigSmallInnerB32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingBigSmallGroup16 {
    inner_a: TwoSiblingBigSmallInnerA16,
    inner_b: TwoSiblingBigSmallInnerB16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingBigSmallOuter32 {
    a: TwoSiblingBigSmallGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingBigSmallOuter16 {
    a: TwoSiblingBigSmallGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigInnerA32 {
    x: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingSmallBigInnerA16 {
    x: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigInnerB32 {
    y: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingSmallBigInnerB16 {
    y: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigGroup32 {
    inner_a: TwoSiblingSmallBigInnerA32,
    inner_b: TwoSiblingSmallBigInnerB32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingSmallBigGroup16 {
    inner_a: TwoSiblingSmallBigInnerA16,
    inner_b: TwoSiblingSmallBigInnerB16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoSiblingSmallBigOuter32 {
    a: TwoSiblingSmallBigGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoSiblingSmallBigOuter16 {
    a: TwoSiblingSmallBigGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingInnerA32 {
    x: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ScalarPlusTwoSiblingInnerA16 {
    x: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingInnerB32 {
    y: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ScalarPlusTwoSiblingInnerB16 {
    y: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingGroup32 {
    a: f32,
    inner_a: ScalarPlusTwoSiblingInnerA32,
    inner_b: ScalarPlusTwoSiblingInnerB32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ScalarPlusTwoSiblingGroup16 {
    a: f32,
    inner_a: ScalarPlusTwoSiblingInnerA16,
    inner_b: ScalarPlusTwoSiblingInnerB16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ScalarPlusTwoSiblingOuter32 {
    a: ScalarPlusTwoSiblingGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ScalarPlusTwoSiblingOuter16 {
    a: ScalarPlusTwoSiblingGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedInnerA32 {
    x: bool,
    y: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ThreeWayMixedInnerA16 {
    x: bool,
    y: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedInnerB32 {
    z: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ThreeWayMixedInnerB16 {
    z: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedGroup32 {
    inner_a: ThreeWayMixedInnerA32,
    a: u8,
    inner_b: ThreeWayMixedInnerB32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ThreeWayMixedGroup16 {
    inner_a: ThreeWayMixedInnerA16,
    a: u8,
    inner_b: ThreeWayMixedInnerB16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ThreeWayMixedOuter32 {
    a: ThreeWayMixedGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ThreeWayMixedOuter16 {
    a: ThreeWayMixedGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedLevel2W32 {
    c: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TripleNestedLevel2W16 {
    c: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedLevel1W32 {
    level2: TripleNestedLevel2W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TripleNestedLevel1W16 {
    level2: TripleNestedLevel2W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedGroup32 {
    level1: TripleNestedLevel1W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TripleNestedGroup16 {
    level1: TripleNestedLevel1W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNestedOuter32 {
    a: TripleNestedGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TripleNestedOuter16 {
    a: TripleNestedGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelDeep32 {
    c: u32,
    d: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedThreeLevelDeep16 {
    c: u32,
    d: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelMid32 {
    b: u16,
    deep: MixedThreeLevelDeep32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedThreeLevelMid16 {
    b: u16,
    deep: MixedThreeLevelDeep16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelGroup32 {
    a: u8,
    mid: MixedThreeLevelMid32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedThreeLevelGroup16 {
    a: u8,
    mid: MixedThreeLevelMid16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedThreeLevelOuter32 {
    a: MixedThreeLevelGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedThreeLevelOuter16 {
    a: MixedThreeLevelGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelL3W32 {
    x: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FourLevelL3W16 {
    x: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelL2W32 {
    l3: FourLevelL3W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FourLevelL2W16 {
    l3: FourLevelL3W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelL1W32 {
    l2: FourLevelL2W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FourLevelL1W16 {
    l2: FourLevelL2W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelGroup32 {
    l1: FourLevelL1W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FourLevelGroup16 {
    l1: FourLevelL1W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FourLevelOuter32 {
    a: FourLevelGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FourLevelOuter16 {
    a: FourLevelGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelL3W32 {
    v: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedFourLevelL3W16 {
    v: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelY32 {
    z: u8,
    l3: MixedFourLevelL3W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedFourLevelY16 {
    z: u8,
    l3: MixedFourLevelL3W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelA32 {
    x: bool,
    y: MixedFourLevelY32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedFourLevelA16 {
    x: bool,
    y: MixedFourLevelY16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelGroup32 {
    a: MixedFourLevelA32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedFourLevelGroup16 {
    a: MixedFourLevelA16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MixedFourLevelOuter32 {
    a: MixedFourLevelGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MixedFourLevelOuter16 {
    a: MixedFourLevelGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL2a32 {
    l3a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoParallelChainsL2a16 {
    l3a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL1a32 {
    l2a: TwoParallelChainsL2a32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoParallelChainsL1a16 {
    l2a: TwoParallelChainsL2a16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL2b32 {
    l3b: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoParallelChainsL2b16 {
    l3b: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsL1b32 {
    l2b: TwoParallelChainsL2b32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoParallelChainsL1b16 {
    l2b: TwoParallelChainsL2b16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsGroup32 {
    l1a: TwoParallelChainsL1a32,
    l1b: TwoParallelChainsL1b32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoParallelChainsGroup16 {
    l1a: TwoParallelChainsL1a16,
    l1b: TwoParallelChainsL1b16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoParallelChainsOuter32 {
    a: TwoParallelChainsGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoParallelChainsOuter16 {
    a: TwoParallelChainsGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FiveFlatPlusNestedInner32 {
    x: bool,
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FiveFlatPlusNestedInner16 {
    x: bool,
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FiveFlatPlusNestedGroup32 {
    a: u64,
    b: f16,
    c: f32,
    d: f64,
    e: Time,
    inner: FiveFlatPlusNestedInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FiveFlatPlusNestedGroup16 {
    a: u64,
    b: f16,
    c: f32,
    d: f64,
    e: Time,
    inner: FiveFlatPlusNestedInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FiveFlatPlusNestedOuter32 {
    a: FiveFlatPlusNestedGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FiveFlatPlusNestedOuter16 {
    a: FiveFlatPlusNestedGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsInner1W32 {
    a: u8,
    b: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoNestedPairsInner1W16 {
    a: u8,
    b: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsInner2W32 {
    c: u32,
    d: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoNestedPairsInner2W16 {
    c: u32,
    d: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsGroup32 {
    inner1: TwoNestedPairsInner1W32,
    inner2: TwoNestedPairsInner2W32,
    e: bool,
    f: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoNestedPairsGroup16 {
    inner1: TwoNestedPairsInner1W16,
    inner2: TwoNestedPairsInner2W16,
    e: bool,
    f: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoNestedPairsOuter32 {
    a: TwoNestedPairsGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoNestedPairsOuter16 {
    a: TwoNestedPairsGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OneScalarPlusBigNestedInner32 {
    b: i16,
    c: i32,
    d: i64,
    e: u8,
    f: u16,
    g: u32,
    h: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OneScalarPlusBigNestedInner16 {
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
struct OneScalarPlusBigNestedGroup32 {
    a: i8,
    inner: OneScalarPlusBigNestedInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OneScalarPlusBigNestedGroup16 {
    a: i8,
    inner: OneScalarPlusBigNestedInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OneScalarPlusBigNestedOuter32 {
    a: OneScalarPlusBigNestedGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OneScalarPlusBigNestedOuter16 {
    a: OneScalarPlusBigNestedGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AllTwelveWrappedInner32 {
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
#[pyro(Word = u16)]
struct AllTwelveWrappedInner16 {
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
struct AllTwelveWrappedGroup32 {
    inner: AllTwelveWrappedInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct AllTwelveWrappedGroup16 {
    inner: AllTwelveWrappedInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AllTwelveWrappedOuter32 {
    a: AllTwelveWrappedGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct AllTwelveWrappedOuter16 {
    a: AllTwelveWrappedGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedSingleFieldBoolInner32 {
    a: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedSingleFieldBoolInner16 {
    a: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedSingleFieldBoolGroup32 {
    inner: NestedSingleFieldBoolInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedSingleFieldBoolGroup16 {
    inner: NestedSingleFieldBoolInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedSingleFieldBoolOuter32 {
    a: NestedSingleFieldBoolGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedSingleFieldBoolOuter16 {
    a: NestedSingleFieldBoolGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedFourIdenticalU32Inner32 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedFourIdenticalU32Inner16 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedFourIdenticalU32Group32 {
    inner: NestedFourIdenticalU32Inner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedFourIdenticalU32Group16 {
    inner: NestedFourIdenticalU32Inner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedFourIdenticalU32Outer32 {
    a: NestedFourIdenticalU32Group32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NestedFourIdenticalU32Outer16 {
    a: NestedFourIdenticalU32Group16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsInner1W32 {
    f_bool: bool,
    f_i8: i8,
    f_i16: i16,
    f_i32: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct SplitThreeSiblingsInner1W16 {
    f_bool: bool,
    f_i8: i8,
    f_i16: i16,
    f_i32: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsInner2W32 {
    f_i64: i64,
    f_u8: u8,
    f_u16: u16,
    f_u32: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct SplitThreeSiblingsInner2W16 {
    f_i64: i64,
    f_u8: u8,
    f_u16: u16,
    f_u32: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsInner3W32 {
    f_u64: u64,
    f_f16: f16,
    f_f32: f32,
    f_f64: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct SplitThreeSiblingsInner3W16 {
    f_u64: u64,
    f_f16: f16,
    f_f32: f32,
    f_f64: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsGroup32 {
    inner_1: SplitThreeSiblingsInner1W32,
    inner_2: SplitThreeSiblingsInner2W32,
    inner_3: SplitThreeSiblingsInner3W32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct SplitThreeSiblingsGroup16 {
    inner_1: SplitThreeSiblingsInner1W16,
    inner_2: SplitThreeSiblingsInner2W16,
    inner_3: SplitThreeSiblingsInner3W16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SplitThreeSiblingsOuter32 {
    a: SplitThreeSiblingsGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct SplitThreeSiblingsOuter16 {
    a: SplitThreeSiblingsGroup16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct EmptyGroup32 {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct EmptyGroup16 {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct EmptyOuter32 {
    a: EmptyGroup32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct EmptyOuter16 {
    a: EmptyGroup16,
}

fn main() {
    assert_pyro_traits::<U8ThenNestedU64Inner32>();
    assert_pyro_traits::<U8ThenNestedU64Inner16>();
    assert_pyro_traits::<U8ThenNestedU64Group32>();
    assert_pyro_traits::<U8ThenNestedU64Group16>();
    assert_pyro_traits::<U8ThenNestedU64Outer32>();
    assert_pyro_traits::<U8ThenNestedU64Outer16>();
    assert_pyro_traits::<NestedU64ThenU8Inner32>();
    assert_pyro_traits::<NestedU64ThenU8Inner16>();
    assert_pyro_traits::<NestedU64ThenU8Group32>();
    assert_pyro_traits::<NestedU64ThenU8Group16>();
    assert_pyro_traits::<NestedU64ThenU8Outer32>();
    assert_pyro_traits::<NestedU64ThenU8Outer16>();
    assert_pyro_traits::<TimestampThenNestedBoolInner32>();
    assert_pyro_traits::<TimestampThenNestedBoolInner16>();
    assert_pyro_traits::<TimestampThenNestedBoolGroup32>();
    assert_pyro_traits::<TimestampThenNestedBoolGroup16>();
    assert_pyro_traits::<TimestampThenNestedBoolOuter32>();
    assert_pyro_traits::<TimestampThenNestedBoolOuter16>();
    assert_pyro_traits::<BoolThenNestedTimestampInner32>();
    assert_pyro_traits::<BoolThenNestedTimestampInner16>();
    assert_pyro_traits::<BoolThenNestedTimestampGroup32>();
    assert_pyro_traits::<BoolThenNestedTimestampGroup16>();
    assert_pyro_traits::<BoolThenNestedTimestampOuter32>();
    assert_pyro_traits::<BoolThenNestedTimestampOuter16>();
    assert_pyro_traits::<NestedTwoThenU16Inner32>();
    assert_pyro_traits::<NestedTwoThenU16Inner16>();
    assert_pyro_traits::<NestedTwoThenU16Group32>();
    assert_pyro_traits::<NestedTwoThenU16Group16>();
    assert_pyro_traits::<NestedTwoThenU16Outer32>();
    assert_pyro_traits::<NestedTwoThenU16Outer16>();
    assert_pyro_traits::<U16ThenNestedTwoInner32>();
    assert_pyro_traits::<U16ThenNestedTwoInner16>();
    assert_pyro_traits::<U16ThenNestedTwoGroup32>();
    assert_pyro_traits::<U16ThenNestedTwoGroup16>();
    assert_pyro_traits::<U16ThenNestedTwoOuter32>();
    assert_pyro_traits::<U16ThenNestedTwoOuter16>();
    assert_pyro_traits::<TwoSiblingBigSmallInnerA32>();
    assert_pyro_traits::<TwoSiblingBigSmallInnerA16>();
    assert_pyro_traits::<TwoSiblingBigSmallInnerB32>();
    assert_pyro_traits::<TwoSiblingBigSmallInnerB16>();
    assert_pyro_traits::<TwoSiblingBigSmallGroup32>();
    assert_pyro_traits::<TwoSiblingBigSmallGroup16>();
    assert_pyro_traits::<TwoSiblingBigSmallOuter32>();
    assert_pyro_traits::<TwoSiblingBigSmallOuter16>();
    assert_pyro_traits::<TwoSiblingSmallBigInnerA32>();
    assert_pyro_traits::<TwoSiblingSmallBigInnerA16>();
    assert_pyro_traits::<TwoSiblingSmallBigInnerB32>();
    assert_pyro_traits::<TwoSiblingSmallBigInnerB16>();
    assert_pyro_traits::<TwoSiblingSmallBigGroup32>();
    assert_pyro_traits::<TwoSiblingSmallBigGroup16>();
    assert_pyro_traits::<TwoSiblingSmallBigOuter32>();
    assert_pyro_traits::<TwoSiblingSmallBigOuter16>();
    assert_pyro_traits::<ScalarPlusTwoSiblingInnerA32>();
    assert_pyro_traits::<ScalarPlusTwoSiblingInnerA16>();
    assert_pyro_traits::<ScalarPlusTwoSiblingInnerB32>();
    assert_pyro_traits::<ScalarPlusTwoSiblingInnerB16>();
    assert_pyro_traits::<ScalarPlusTwoSiblingGroup32>();
    assert_pyro_traits::<ScalarPlusTwoSiblingGroup16>();
    assert_pyro_traits::<ScalarPlusTwoSiblingOuter32>();
    assert_pyro_traits::<ScalarPlusTwoSiblingOuter16>();
    assert_pyro_traits::<ThreeWayMixedInnerA32>();
    assert_pyro_traits::<ThreeWayMixedInnerA16>();
    assert_pyro_traits::<ThreeWayMixedInnerB32>();
    assert_pyro_traits::<ThreeWayMixedInnerB16>();
    assert_pyro_traits::<ThreeWayMixedGroup32>();
    assert_pyro_traits::<ThreeWayMixedGroup16>();
    assert_pyro_traits::<ThreeWayMixedOuter32>();
    assert_pyro_traits::<ThreeWayMixedOuter16>();
    assert_pyro_traits::<TripleNestedLevel2W32>();
    assert_pyro_traits::<TripleNestedLevel2W16>();
    assert_pyro_traits::<TripleNestedLevel1W32>();
    assert_pyro_traits::<TripleNestedLevel1W16>();
    assert_pyro_traits::<TripleNestedGroup32>();
    assert_pyro_traits::<TripleNestedGroup16>();
    assert_pyro_traits::<TripleNestedOuter32>();
    assert_pyro_traits::<TripleNestedOuter16>();
    assert_pyro_traits::<MixedThreeLevelDeep32>();
    assert_pyro_traits::<MixedThreeLevelDeep16>();
    assert_pyro_traits::<MixedThreeLevelMid32>();
    assert_pyro_traits::<MixedThreeLevelMid16>();
    assert_pyro_traits::<MixedThreeLevelGroup32>();
    assert_pyro_traits::<MixedThreeLevelGroup16>();
    assert_pyro_traits::<MixedThreeLevelOuter32>();
    assert_pyro_traits::<MixedThreeLevelOuter16>();
    assert_pyro_traits::<FourLevelL3W32>();
    assert_pyro_traits::<FourLevelL3W16>();
    assert_pyro_traits::<FourLevelL2W32>();
    assert_pyro_traits::<FourLevelL2W16>();
    assert_pyro_traits::<FourLevelL1W32>();
    assert_pyro_traits::<FourLevelL1W16>();
    assert_pyro_traits::<FourLevelGroup32>();
    assert_pyro_traits::<FourLevelGroup16>();
    assert_pyro_traits::<FourLevelOuter32>();
    assert_pyro_traits::<FourLevelOuter16>();
    assert_pyro_traits::<MixedFourLevelL3W32>();
    assert_pyro_traits::<MixedFourLevelL3W16>();
    assert_pyro_traits::<MixedFourLevelY32>();
    assert_pyro_traits::<MixedFourLevelY16>();
    assert_pyro_traits::<MixedFourLevelA32>();
    assert_pyro_traits::<MixedFourLevelA16>();
    assert_pyro_traits::<MixedFourLevelGroup32>();
    assert_pyro_traits::<MixedFourLevelGroup16>();
    assert_pyro_traits::<MixedFourLevelOuter32>();
    assert_pyro_traits::<MixedFourLevelOuter16>();
    assert_pyro_traits::<TwoParallelChainsL2a32>();
    assert_pyro_traits::<TwoParallelChainsL2a16>();
    assert_pyro_traits::<TwoParallelChainsL1a32>();
    assert_pyro_traits::<TwoParallelChainsL1a16>();
    assert_pyro_traits::<TwoParallelChainsL2b32>();
    assert_pyro_traits::<TwoParallelChainsL2b16>();
    assert_pyro_traits::<TwoParallelChainsL1b32>();
    assert_pyro_traits::<TwoParallelChainsL1b16>();
    assert_pyro_traits::<TwoParallelChainsGroup32>();
    assert_pyro_traits::<TwoParallelChainsGroup16>();
    assert_pyro_traits::<TwoParallelChainsOuter32>();
    assert_pyro_traits::<TwoParallelChainsOuter16>();
    assert_pyro_traits::<FiveFlatPlusNestedInner32>();
    assert_pyro_traits::<FiveFlatPlusNestedInner16>();
    assert_pyro_traits::<FiveFlatPlusNestedGroup32>();
    assert_pyro_traits::<FiveFlatPlusNestedGroup16>();
    assert_pyro_traits::<FiveFlatPlusNestedOuter32>();
    assert_pyro_traits::<FiveFlatPlusNestedOuter16>();
    assert_pyro_traits::<TwoNestedPairsInner1W32>();
    assert_pyro_traits::<TwoNestedPairsInner1W16>();
    assert_pyro_traits::<TwoNestedPairsInner2W32>();
    assert_pyro_traits::<TwoNestedPairsInner2W16>();
    assert_pyro_traits::<TwoNestedPairsGroup32>();
    assert_pyro_traits::<TwoNestedPairsGroup16>();
    assert_pyro_traits::<TwoNestedPairsOuter32>();
    assert_pyro_traits::<TwoNestedPairsOuter16>();
    assert_pyro_traits::<OneScalarPlusBigNestedInner32>();
    assert_pyro_traits::<OneScalarPlusBigNestedInner16>();
    assert_pyro_traits::<OneScalarPlusBigNestedGroup32>();
    assert_pyro_traits::<OneScalarPlusBigNestedGroup16>();
    assert_pyro_traits::<OneScalarPlusBigNestedOuter32>();
    assert_pyro_traits::<OneScalarPlusBigNestedOuter16>();
    assert_pyro_traits::<AllTwelveWrappedInner32>();
    assert_pyro_traits::<AllTwelveWrappedInner16>();
    assert_pyro_traits::<AllTwelveWrappedGroup32>();
    assert_pyro_traits::<AllTwelveWrappedGroup16>();
    assert_pyro_traits::<AllTwelveWrappedOuter32>();
    assert_pyro_traits::<AllTwelveWrappedOuter16>();
    assert_pyro_traits::<NestedSingleFieldBoolInner32>();
    assert_pyro_traits::<NestedSingleFieldBoolInner16>();
    assert_pyro_traits::<NestedSingleFieldBoolGroup32>();
    assert_pyro_traits::<NestedSingleFieldBoolGroup16>();
    assert_pyro_traits::<NestedSingleFieldBoolOuter32>();
    assert_pyro_traits::<NestedSingleFieldBoolOuter16>();
    assert_pyro_traits::<NestedFourIdenticalU32Inner32>();
    assert_pyro_traits::<NestedFourIdenticalU32Inner16>();
    assert_pyro_traits::<NestedFourIdenticalU32Group32>();
    assert_pyro_traits::<NestedFourIdenticalU32Group16>();
    assert_pyro_traits::<NestedFourIdenticalU32Outer32>();
    assert_pyro_traits::<NestedFourIdenticalU32Outer16>();
    assert_pyro_traits::<SplitThreeSiblingsInner1W32>();
    assert_pyro_traits::<SplitThreeSiblingsInner1W16>();
    assert_pyro_traits::<SplitThreeSiblingsInner2W32>();
    assert_pyro_traits::<SplitThreeSiblingsInner2W16>();
    assert_pyro_traits::<SplitThreeSiblingsInner3W32>();
    assert_pyro_traits::<SplitThreeSiblingsInner3W16>();
    assert_pyro_traits::<SplitThreeSiblingsGroup32>();
    assert_pyro_traits::<SplitThreeSiblingsGroup16>();
    assert_pyro_traits::<SplitThreeSiblingsOuter32>();
    assert_pyro_traits::<SplitThreeSiblingsOuter16>();
    assert_pyro_traits::<EmptyGroup32>();
    assert_pyro_traits::<EmptyGroup16>();
    assert_pyro_traits::<EmptyOuter32>();
    assert_pyro_traits::<EmptyOuter16>();
}
