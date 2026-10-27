use crate::schema::PyroType;

pub mod readers;
pub mod value_readers;
pub mod value_writers;
pub mod writers;

pub trait IoType {
    /// Returns the schema field this writer is responsible for.
    fn datatype(&self) -> PyroType;
}

// Gated on `feature = "fixtures"` (in addition to the usual `test`) so
// every hand-built value/wire test's golden `expected_buf`/`expected_inodes`
// consts are reachable as `pub` items from `tests/` — see the `fixtures`
// feature's doc comment in Cargo.toml. `tracing_test::traced_test` (a
// dev-dependency) is only ever attached via `#[cfg_attr(test, ...)]` inside
// these files precisely so a plain `--features fixtures` build (no
// `--test`) never needs it resolvable.
#[cfg(any(test, feature = "fixtures"))]
pub mod tests {
    pub mod basic_1_nesting_u16;
    pub mod basic_1_nesting_u32;
    pub mod basic_2_nesting_u16;
    pub mod basic_2_nesting_u32;
    pub mod basic_3_nesting_u16;
    pub mod basic_3_nesting_u32;
    pub mod basic_4_nesting_u16;
    pub mod basic_4_nesting_u32;
    pub mod basic_nesting_u16;
    pub mod basic_nesting_u32;
    pub mod basic_u16;
    pub mod basic_u32;
    pub mod bench_u16;
    pub mod bench_u32;
    pub mod difficult_u16;
    pub mod difficult_u32;
    pub mod empty_groups;
    pub mod fixed_u16;
    pub mod fixed_u32;
    pub mod group_1_nesting_u16;
    pub mod group_1_nesting_u32;
    pub mod group_2_nesting_u16;
    pub mod group_2_nesting_u32;
    pub mod group_3_nesting_u16;
    pub mod group_3_nesting_u32;
    pub mod group_4_nesting_u16;
    pub mod group_4_nesting_u32;
    pub mod group_5_nesting_u16;
    pub mod group_5_nesting_u32;
    pub mod group_nesting_u16;
    pub mod group_nesting_u32;
    pub mod group_option_1_nesting_u16;
    pub mod group_option_1_nesting_u32;
    pub mod group_option_2_nesting_u16;
    pub mod group_option_2_nesting_u32;
    pub mod group_option_3_nesting_u16;
    pub mod group_option_3_nesting_u32;
    pub mod group_option_4_nesting_u16;
    pub mod group_option_4_nesting_u32;
    pub mod group_option_5_nesting_u16;
    pub mod group_option_5_nesting_u32;
    pub mod group_option_nesting_u16;
    pub mod group_option_nesting_u32;
    pub mod map_nested_u16;
    pub mod map_nested_u32;
    pub mod map_u16;
    pub mod map_u32;
    pub mod primitive_1_fixed_nesting_u16;
    pub mod primitive_1_fixed_nesting_u32;
    pub mod primitive_1_nesting_u16;
    pub mod primitive_1_nesting_u32;
    pub mod primitive_1_option_fixed_nesting_u16;
    pub mod primitive_1_option_fixed_nesting_u32;
    pub mod primitive_1_option_nesting_u16;
    pub mod primitive_1_option_nesting_u32;
    pub mod primitive_2_fixed_nesting_u16;
    pub mod primitive_2_fixed_nesting_u32;
    pub mod primitive_2_nesting_u16;
    pub mod primitive_2_nesting_u32;
    pub mod primitive_2_option_fixed_nesting_u16;
    pub mod primitive_2_option_fixed_nesting_u32;
    pub mod primitive_2_option_nesting_u16;
    pub mod primitive_2_option_nesting_u32;
    pub mod primitive_3_fixed_nesting_u16;
    pub mod primitive_3_fixed_nesting_u32;
    pub mod primitive_3_nesting_u16;
    pub mod primitive_3_nesting_u32;
    pub mod primitive_3_option_fixed_nesting_u16;
    pub mod primitive_3_option_fixed_nesting_u32;
    pub mod primitive_3_option_nesting_u16;
    pub mod primitive_3_option_nesting_u32;
    pub mod primitive_4_fixed_nesting_u16;
    pub mod primitive_4_fixed_nesting_u32;
    pub mod primitive_4_nesting_u16;
    pub mod primitive_4_nesting_u32;
    pub mod primitive_4_option_fixed_nesting_u16;
    pub mod primitive_4_option_fixed_nesting_u32;
    pub mod primitive_4_option_nesting_u16;
    pub mod primitive_4_option_nesting_u32;
    pub mod primitive_5_fixed_nesting_u16;
    pub mod primitive_5_fixed_nesting_u32;
    pub mod primitive_5_nesting_u16;
    pub mod primitive_5_nesting_u32;
    pub mod primitive_5_option_fixed_nesting_u16;
    pub mod primitive_5_option_fixed_nesting_u32;
    pub mod primitive_5_option_nesting_u16;
    pub mod primitive_5_option_nesting_u32;
    pub mod primitive_group_option_u16;
    pub mod primitive_group_option_u32;
    pub mod primitive_group_u16;
    pub mod primitive_group_u32;
    pub mod primitive_option_u16;
    pub mod primitive_option_u32;
    pub mod primitive_slice_option_u16;
    pub mod primitive_slice_option_u32;
    pub mod primitive_slice_u16;
    pub mod primitive_slice_u32;
    pub mod primitive_u16;
    pub mod primitive_u32;
    pub mod support;
}
