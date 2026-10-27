use std::fmt;

use crate::{
    PyroRow,
    value::{PrimitiveValueList, PyroValue},
};

// -----------------------------------------------------------------------------
// Display Configuration
// -----------------------------------------------------------------------------

const MAX_DISPLAY_ITEMS: usize = 10;
// How many items to show at the start before '...'
const HEAD_DISPLAY_ITEMS: usize = 8;
// How many items to show at the end after '...'
const TAIL_DISPLAY_ITEMS: usize = 2;

// -----------------------------------------------------------------------------
// PyroValue Display
// -----------------------------------------------------------------------------

impl<'a> fmt::Display for PyroValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PyroValue::Null => write!(f, "null"),
            PyroValue::Bool(v) => write!(f, "{v}"),

            // Numbers
            PyroValue::I8(v) => write!(f, "{v}"),
            PyroValue::I16(v) => write!(f, "{v}"),
            PyroValue::I32(v) => write!(f, "{v}"),
            PyroValue::I64(v) => write!(f, "{v}"),
            PyroValue::U8(v) => write!(f, "{v}"),
            PyroValue::U16(v) => write!(f, "{v}"),
            PyroValue::U32(v) => write!(f, "{v}"),
            PyroValue::U64(v) => write!(f, "{v}"),
            PyroValue::F16(v) => write!(f, "{v}"),
            PyroValue::F32(v) => write!(f, "{v}"),
            PyroValue::F64(v) => write!(f, "{v}"),

            // Use Debug for Str to include quotes ("value") -> safer for data visualization
            PyroValue::Str(v) => write!(f, "{v:?}"),
            PyroValue::Bytes(v) => write!(f, "{v:?}"),

            PyroValue::Timestamp(nanos) => {
                write!(f, "({nanos})")
            }

            // Complex Types
            PyroValue::PrimitiveList(pl) => write!(f, "{pl}"),

            PyroValue::List(items) => fmt_truncated_slice(f, items),

            PyroValue::Group(row) => {
                // Delegate to helper to format the inner Vec<RowItem>
                write!(f, "{{")?;
                fmt_truncated_group(f, &row)?;
                write!(f, "}}")
            }

            PyroValue::Map(entries) => {
                write!(f, "{{")?;
                fmt_truncated_map(f, entries.key_slice(), entries.value_slice(), |f, k, v| {
                    write!(f, "{k} => {v}")
                })?;
                write!(f, "}}")
            }
        }
    }
}

// -----------------------------------------------------------------------------
// PrimitiveValueList Display
// -----------------------------------------------------------------------------

impl<'a> fmt::Display for PrimitiveValueList<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveValueList::Bool(v) => {
                let bools: Vec<bool> = v.iter().map(|b| *b).collect();
                fmt_truncated_slice(f, &bools)
            }
            PrimitiveValueList::U8(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::U16(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::U32(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::U64(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::I8(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::I16(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::I32(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::I64(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::F16(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::F32(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::F64(v) => fmt_truncated_slice(f, v),
            PrimitiveValueList::Timestamp(v) => fmt_truncated_slice(f, v),
        }
    }
}

// -----------------------------------------------------------------------------
// Formatting Helpers
// -----------------------------------------------------------------------------

/// Formats a slice `[T]` with truncation: `[v1, v2, ..., vN-1, vN]`
fn fmt_truncated_slice<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    write!(f, "[")?;
    let len = items.len();

    if len <= MAX_DISPLAY_ITEMS {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{item}")?;
        }
    } else {
        // HEAD: First 8 items
        for (i, item) in items.iter().enumerate().take(HEAD_DISPLAY_ITEMS) {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{item}")?;
        }

        // ELLIPSIS
        let omitted = len - HEAD_DISPLAY_ITEMS - TAIL_DISPLAY_ITEMS;
        write!(f, ", ... ({omitted} omitted), ")?;

        // TAIL: Last 2 items
        for (i, item) in items
            .iter()
            .enumerate()
            .take(len)
            .skip(len - TAIL_DISPLAY_ITEMS)
        {
            write!(f, "{item}")?;
            if i < len - 1 {
                write!(f, ", ")?;
            }
        }
    }
    write!(f, "]")
}

/// Formats a list of Map/Struct entries with truncation.
/// `formatter` closure handles formatting the individual Key-Value pair.
fn fmt_truncated_map<K, V, F>(
    f: &mut fmt::Formatter<'_>,
    keys: &[K],
    values: &[V],
    formatter: F,
) -> fmt::Result
where
    F: Fn(&mut fmt::Formatter<'_>, &K, &V) -> fmt::Result,
{
    let len = keys.len();

    if len <= MAX_DISPLAY_ITEMS {
        for (i, (key, value)) in keys.iter().zip(values.iter()).enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            formatter(f, key, value)?;
        }
    } else {
        for (i, (key, value)) in keys
            .iter()
            .zip(values.iter())
            .enumerate()
            .take(HEAD_DISPLAY_ITEMS)
        {
            if i > 0 {
                write!(f, ", ")?;
            }
            formatter(f, key, value)?;
        }

        let omitted = len - HEAD_DISPLAY_ITEMS - TAIL_DISPLAY_ITEMS;
        write!(f, ", ... ({omitted} fields omitted), ")?;

        for (i, (key, value)) in keys
            .iter()
            .zip(values.iter())
            .enumerate()
            .take(len)
            .skip(len - TAIL_DISPLAY_ITEMS)
        {
            formatter(f, key, value)?;
            if i < len - 1 {
                write!(f, ", ")?;
            }
        }
    }
    Ok(())
}

/// Formats a list of Map/Struct entries with truncation.
/// `formatter` closure handles formatting the individual Key-Value pair.
fn fmt_truncated_group(f: &mut fmt::Formatter<'_>, keys: &PyroRow<'_>) -> fmt::Result {
    let len = keys.len();

    if len <= MAX_DISPLAY_ITEMS {
        for (i, (key, value)) in keys.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", key, value)?;
        }
    } else {
        for (i, (key, value)) in keys.iter().enumerate().take(HEAD_DISPLAY_ITEMS) {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", key, value)?;
        }

        let omitted = len - HEAD_DISPLAY_ITEMS - TAIL_DISPLAY_ITEMS;
        write!(f, ", ... ({omitted} fields omitted), ")?;

        for (i, (key, value)) in keys
            .iter()
            .enumerate()
            .take(len)
            .skip(len - TAIL_DISPLAY_ITEMS)
        {
            write!(f, "{}: {}", key, value)?;
            if i < len - 1 {
                write!(f, ", ")?;
            }
        }
    }
    Ok(())
}
