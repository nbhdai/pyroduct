// arrow_row_display.rs

use crate::{ArrowRow, ArrowValue, PrimitiveValueList};
use std::fmt;

/// Controls how the ArrowRow is formatted for display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowDisplayMode {
    Verbose,
    Compact,
}

/// A wrapper struct to enable `fmt::Display` for ArrowRow with a specific mode.
pub struct RowPrinter<'a> {
    row: &'a ArrowRow<'a>,
    mode: RowDisplayMode,
}

impl<'a> RowPrinter<'a> {
    pub fn new(row: &'a ArrowRow<'a>, mode: RowDisplayMode) -> Self {
        Self { row, mode }
    }
}

impl<'a> fmt::Display for RowPrinter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            RowDisplayMode::Verbose => print_verbose(self.row, f, 0),
            RowDisplayMode::Compact => print_compact(self.row, f),
        }
    }
}

// -----------------------------------------------------------------------------
// Verbose Formatter (Key: Value (Type))
// -----------------------------------------------------------------------------

fn print_verbose(row: &ArrowRow, f: &mut fmt::Formatter<'_>, indent_level: usize) -> fmt::Result {
    let indent = "  ".repeat(indent_level);

    for (key, value) in row.iter() {
        write!(f, "{}{}: ", indent, key)?;
        fmt_value_verbose(value, f, indent_level)?;
        writeln!(f)?;
    }
    Ok(())
}

fn fmt_value_verbose(
    val: &ArrowValue,
    f: &mut fmt::Formatter<'_>,
    indent_level: usize,
) -> fmt::Result {
    match val {
        ArrowValue::Null => write!(f, "null (Null)"),
        ArrowValue::Bool(v) => write!(f, "{} (Bool)", v),
        ArrowValue::I8(v) => write!(f, "{} (Int8)", v),
        ArrowValue::I16(v) => write!(f, "{} (Int16)", v),
        ArrowValue::I32(v) => write!(f, "{} (Int32)", v),
        ArrowValue::I64(v) => write!(f, "{} (Int64)", v),
        ArrowValue::U8(v) => write!(f, "{} (UInt8)", v),
        ArrowValue::U16(v) => write!(f, "{} (UInt16)", v),
        ArrowValue::U32(v) => write!(f, "{} (UInt32)", v),
        ArrowValue::U64(v) => write!(f, "{} (UInt64)", v),
        ArrowValue::F16(v) => write!(f, "{} (Float16)", v),
        ArrowValue::F32(v) => write!(f, "{} (Float32)", v),
        ArrowValue::F64(v) => write!(f, "{} (Float64)", v),
        ArrowValue::IntervalDayTime { days, milliseconds } => {
            write!(f, "{}d {}ms (IntervalDayTime)", days, milliseconds)
        }
        ArrowValue::Str(v) => write!(f, "\"{}\" (String)", v),

        // Complex Types
        ArrowValue::PrimitiveList(list) => {
            write!(f, "[")?;
            // Preview first few items to avoid spamming the CLI
            let max_preview = 5;
            let len = list.len();

            // Helper to print primitive slices generic over T
            fn print_slice<T: fmt::Display>(
                f: &mut fmt::Formatter,
                slice: &[T],
                max: usize,
            ) -> fmt::Result {
                for (i, item) in slice.iter().take(max).enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                Ok(())
            }

            match list {
                PrimitiveValueList::Bool(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::U8(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::U16(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::U32(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::U64(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::I8(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::I16(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::I32(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::I64(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::F16(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::F32(v) => print_slice(f, v, max_preview)?,
                PrimitiveValueList::F64(v) => print_slice(f, v, max_preview)?,
            }

            if len > max_preview {
                write!(f, ", ... {} more", len - max_preview)?;
            }
            write!(f, "] (PrimitiveList[{:?}])", list_type_name(list))
        }

        ArrowValue::Group(row) => {
            writeln!(f, "(Group)")?;
            // Recursive call with increased indent
            print_verbose(row, f, indent_level + 1)?;
            // Backtrack cursor slightly to align with parent if needed,
            // but standard recursion usually handles this by finishing the previous writeln.
            Ok(())
        }

        ArrowValue::List(items) => {
            writeln!(f, "(List)")?;
            let next_indent = "  ".repeat(indent_level + 1);
            for (i, item) in items.iter().enumerate() {
                write!(f, "{}[{}]: ", next_indent, i)?;
                fmt_value_verbose(item, f, indent_level + 1)?;
                writeln!(f)?;
            }
            Ok(())
        }

        ArrowValue::MapInternal(items) => {
            writeln!(f, "(Map)")?;
            let next_indent = "  ".repeat(indent_level + 1);
            for (k, v) in items {
                write!(f, "{}[", next_indent)?;
                fmt_value_compact(k, f)?; // Keys usually short
                write!(f, "] => ")?;
                fmt_value_verbose(v, f, indent_level + 1)?;
                writeln!(f)?;
            }
            Ok(())
        }
    }
}

fn list_type_name(l: &PrimitiveValueList) -> &'static str {
    match l {
        PrimitiveValueList::Bool(_) => "Bool",
        PrimitiveValueList::U8(_) => "U8",
        PrimitiveValueList::U16(_) => "U16",
        PrimitiveValueList::U32(_) => "U32",
        PrimitiveValueList::U64(_) => "U64",
        PrimitiveValueList::I8(_) => "I8",
        PrimitiveValueList::I16(_) => "I16",
        PrimitiveValueList::I32(_) => "I32",
        PrimitiveValueList::I64(_) => "I64",
        PrimitiveValueList::F16(_) => "F16",
        PrimitiveValueList::F32(_) => "F32",
        PrimitiveValueList::F64(_) => "F64",
    }
}

// -----------------------------------------------------------------------------
// Compact Formatter (Value, Value, ...)
// -----------------------------------------------------------------------------

fn print_compact(row: &ArrowRow, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, val) in row.values().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        fmt_value_compact(val, f)?;
    }
    Ok(())
}

fn fmt_value_compact(val: &ArrowValue, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match val {
        ArrowValue::Null => write!(f, "null"),
        ArrowValue::Bool(v) => write!(f, "{}", v),
        ArrowValue::I8(v) => write!(f, "{}", v),
        ArrowValue::I16(v) => write!(f, "{}", v),
        ArrowValue::I32(v) => write!(f, "{}", v),
        ArrowValue::I64(v) => write!(f, "{}", v),
        ArrowValue::U8(v) => write!(f, "{}", v),
        ArrowValue::U16(v) => write!(f, "{}", v),
        ArrowValue::U32(v) => write!(f, "{}", v),
        ArrowValue::U64(v) => write!(f, "{}", v),
        ArrowValue::F16(v) => write!(f, "{}", v),
        ArrowValue::F32(v) => write!(f, "{}", v),
        ArrowValue::F64(v) => write!(f, "{}", v),
        ArrowValue::IntervalDayTime { days, milliseconds } => {
            write!(f, "{}d{}ms", days, milliseconds)
        }
        ArrowValue::Str(v) => write!(f, "{}", v), // No quotes in compact mode usually cleaner for tables

        // Condensed representations for complex types
        ArrowValue::PrimitiveList(l) => write!(f, "[{} items]", l.len()),
        ArrowValue::Group(g) => {
            write!(f, "{{")?;
            print_compact(g, f)?;
            write!(f, "}}")
        }
        ArrowValue::List(l) => {
            write!(f, "[")?;
            for (i, item) in l.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                fmt_value_compact(item, f)?;
            }
            write!(f, "]")
        }
        ArrowValue::MapInternal(m) => write!(f, "{{Map: {} items}}", m.len()),
    }
}
