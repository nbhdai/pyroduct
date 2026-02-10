// arrow_row_display.rs

use super::{PyroRow, PyroValue, PrimitiveValueList};
use std::fmt;

impl fmt::Display for PyroRow<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        RowPrinter::new(self, RowDisplayMode::Compact).fmt(f)
    }
}

/// Controls how the PyroRow is formatted for display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowDisplayMode {
    Verbose,
    Compact,
}

/// A wrapper struct to enable `fmt::Display` for PyroRow with a specific mode.
pub struct RowPrinter<'a> {
    row: &'a PyroRow<'a>,
    mode: RowDisplayMode,
}

impl<'a> RowPrinter<'a> {
    pub fn new(row: &'a PyroRow<'a>, mode: RowDisplayMode) -> Self {
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

fn print_verbose(row: &PyroRow, f: &mut fmt::Formatter<'_>, indent_level: usize) -> fmt::Result {
    let indent = "  ".repeat(indent_level);

    for (key, value) in row.iter() {
        write!(f, "{}{}: ", indent, key)?;
        fmt_value_verbose(value, f, indent_level)?;
        writeln!(f)?;
    }
    Ok(())
}

fn fmt_value_verbose(
    val: &PyroValue,
    f: &mut fmt::Formatter<'_>,
    indent_level: usize,
) -> fmt::Result {
    // We delegate the formatting of the value itself to the Display implementation in display.rs.
    // We only handle the Type annotation and hierarchical indentation here.
    match val {
        PyroValue::Null => write!(f, "{} (Null)", val),
        PyroValue::Bool(_) => write!(f, "{} (Bool)", val),

        PyroValue::I8(_) => write!(f, "{} (Int8)", val),
        PyroValue::I16(_) => write!(f, "{} (Int16)", val),
        PyroValue::I32(_) => write!(f, "{} (Int32)", val),
        PyroValue::I64(_) => write!(f, "{} (Int64)", val),

        PyroValue::U8(_) => write!(f, "{} (UInt8)", val),
        PyroValue::U16(_) => write!(f, "{} (UInt16)", val),
        PyroValue::U32(_) => write!(f, "{} (UInt32)", val),
        PyroValue::U64(_) => write!(f, "{} (UInt64)", val),

        PyroValue::F16(_) => write!(f, "{} (Float16)", val),
        PyroValue::F32(_) => write!(f, "{} (Float32)", val),
        PyroValue::F64(_) => write!(f, "{} (Float64)", val),

        PyroValue::Timestamp { .. } => {
            write!(f, "{} (IntervalDayTime)", val)
        }

        // display.rs formatting for Str uses Debug (adds quotes), which is perfect for verbose.
        PyroValue::Str(_) => write!(f, "{} (String)", val),

        // Complex Types

        // display.rs handles truncation logic (e.g. [1, 2, ...]).
        PyroValue::PrimitiveList(list) => {
            write!(f, "{} (PrimitiveList[{}])", val, list_type_name(list))
        }

        // For structural types (Group, List, Map), we keep the recursive indentation logic
        // of Verbose mode, rather than using the inline format from display.rs.
        PyroValue::Group(row) => {
            writeln!(f, "(Group)")?;
            print_verbose(row, f, indent_level + 1)?;
            Ok(())
        }

        PyroValue::List(items) => {
            writeln!(f, "(List)")?;
            let next_indent = "  ".repeat(indent_level + 1);
            for (i, item) in items.iter().enumerate() {
                write!(f, "{}[{}]: ", next_indent, i)?;
                fmt_value_verbose(item, f, indent_level + 1)?;
                writeln!(f)?;
            }
            Ok(())
        }

        PyroValue::MapInternal(items) => {
            writeln!(f, "(Map)")?;
            let next_indent = "  ".repeat(indent_level + 1);
            for (k, v) in items {
                write!(f, "{}[", next_indent)?;
                // Keys use standard display (likely string or primitive)
                write!(f, "{}", k)?;
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

fn print_compact(row: &PyroRow, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Relies entirely on the fmt::Display implementation in display.rs
    // for value formatting (including truncation and quotes).
    for (i, val) in row.values().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}", val)?;
    }
    Ok(())
}
