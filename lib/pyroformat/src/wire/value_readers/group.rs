use bitvec::{order::Lsb0, slice::BitSlice};
use pyro_schema::{Bow, FieldName};

use crate::{
    MAX_TIERS, PyroRow, PyroValue,
    errors::WireError,
    schema::PyroType,
    value::RowItem,
    wire::value_readers::{Reader, SplitSource, ValueNestedReader, ValueReader},
};
use pyro_schema::StructureWord;

#[derive(Debug, Clone)]
pub struct GroupReader {
    n: usize,
    depth: usize,
    elements: Vec<(FieldName<'static>, Reader)>,
}

#[derive(Debug, Clone)]
pub struct GroupOptionReader {
    n: usize,
    depth: usize,
    elements: Vec<(FieldName<'static>, Reader)>,
}

/// One field's column: every row's value for it, in order.
type Column<'a> = (FieldName<'a>, std::vec::IntoIter<PyroValue<'a>>);

/// Every field's column, each checked to hold `rows` values.
fn columns<'a, W: StructureWord>(
    elements: &[(FieldName<'static>, Reader)],
    buffer: &SplitSource<'a, W>,
    rows: usize,
) -> Result<Vec<Column<'a>>, WireError> {
    elements
        .iter()
        .enumerate()
        .map(|(index, (name, reader))| {
            let values = reader.read_values(buffer)?;
            if values.len() != rows {
                return Err(WireError::LengthMismatch {
                    expected: rows,
                    have: values.len(),
                    index,
                });
            }
            Ok((name.clone(), values.into_iter()))
        })
        .collect()
}

/// The next row: one value off every column.
fn next_row<'a>(columns: &mut [Column<'a>]) -> Option<PyroValue<'a>> {
    let mut row = Vec::with_capacity(columns.len());
    for (name, values) in columns.iter_mut() {
        row.push(RowItem {
            key: name.clone(),
            value: values.next()?,
        });
    }
    Some(PyroValue::Group(PyroRow(row)))
}

/// Every row of a group column, as `PyroValue::Group`s.
pub struct GroupIter<'a> {
    rows: usize,
    columns: Vec<Column<'a>>,
}

impl<'a> Iterator for GroupIter<'a> {
    type Item = PyroValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rows == 0 {
            return None;
        }
        self.rows -= 1;
        next_row(&mut self.columns)
    }
}

/// Every sample of a nullable group column: a `PyroValue::Group` per set
/// bit of `mask`, taken from the columns (which hold only the present
/// rows), and a `PyroValue::Null` per clear one.
pub struct GroupMaskedIter<'a> {
    mask: &'a BitSlice<u8, Lsb0>,
    idx: usize,
    columns: Vec<Column<'a>>,
}

impl<'a> Iterator for GroupMaskedIter<'a> {
    type Item = PyroValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let present = *self.mask.get(self.idx)?;
        self.idx += 1;
        if !present {
            // An absent group: its children were never fed, so nothing is
            // taken — but the entry itself is still there, as a `Null`.
            return Some(PyroValue::Null);
        }
        next_row(&mut self.columns)
    }
}

/// Wrap `t` in `lists` layers of `List`.
fn nested(mut t: PyroType, lists: usize) -> PyroType {
    for _ in 0..lists {
        t = PyroType::List(Bow::new(t));
    }
    t
}

/// One bare group: every field's own value.
fn read_row<'a, W: StructureWord>(
    elements: &[(FieldName<'static>, Reader)],
    buffer: &SplitSource<'a, W>,
) -> Result<PyroValue<'a>, WireError> {
    let row = elements
        .iter()
        .map(|(name, reader)| {
            Ok(RowItem {
                key: name.clone(),
                value: reader.read_value(buffer)?,
            })
        })
        .collect::<Result<Vec<_>, WireError>>()?;
    Ok(PyroValue::Group(PyroRow(row)))
}

impl GroupReader {
    /// The read-side twin of `wire::value_writers::group::Group::new` *and*
    /// `NestedGroup::new` — the reader has a single `GroupReader` where the
    /// writer splits flat from repeated, so the flat/nested choice is just
    /// `depth == 0`.
    ///
    /// `depth` = `nesting + group_nest`, i.e. one *more* than the writer's
    /// `NestedGroup` depth (`nesting - 1 + group_nest`), because the reader
    /// counts the group's own instance level: a flat group is `0`, a root
    /// `List<Group>` and a group field of a repeated group are both `1`.
    /// At `1` the slot holds the row count; every level above that is one
    /// topology level.
    pub fn new(n: usize, depth: usize, elements: Vec<(FieldName<'static>, Reader)>) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        tracing::trace!(n, depth, fields = elements.len(), "Creating GroupReader");
        Self { n, depth, elements }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn elements(&self) -> &[(FieldName<'static>, Reader)] {
        &self.elements
    }

    pub fn datatype(&self) -> PyroType {
        let group = PyroType::Group(self.elements.iter().map(|(_, r)| r.datatype()).collect());
        nested(group, self.depth.saturating_sub(1))
    }
}

impl GroupOptionReader {
    /// The read-side twin of `wire::value_writers::group::OptionGroup::new`
    /// and `NestedOptionGroup::new`; see [`GroupReader::new`] for the `depth`
    /// convention.
    ///
    /// A flat nullable group (`depth == 0`) is present when inode `n`'s bit
    /// in the fixed region's bitmap is set. A repeated one carries its own
    /// per-sample mask: at `1` the slot is `(mask start, slots)`; above
    /// that the mask sits right behind the topology words, over the last
    /// level's total.
    pub fn new(n: usize, depth: usize, elements: Vec<(FieldName<'static>, Reader)>) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        tracing::trace!(
            n,
            depth,
            fields = elements.len(),
            "Creating GroupOptionReader"
        );
        Self { n, depth, elements }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn elements(&self) -> &[(FieldName<'static>, Reader)] {
        &self.elements
    }

    pub fn datatype(&self) -> PyroType {
        let group =
            PyroType::OptionGroup(self.elements.iter().map(|(_, r)| r.datatype()).collect());
        nested(group, self.depth.saturating_sub(1))
    }
}

impl ValueReader for GroupReader {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_value<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroValue<'a>, WireError> {
        match self.depth {
            0 => read_row(&self.elements, buffer),
            _ => Ok(PyroValue::List(self.read_values(buffer)?)),
        }
    }
}

impl ValueNestedReader for GroupReader {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_values<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<Vec<PyroValue<'a>>, WireError> {
        match self.depth {
            // A bare group is one value, not a column of them.
            0 => Err(WireError::InvalidInodes),
            1 => {
                let rows = buffer.read_count(self.n);
                let columns = columns(&self.elements, buffer, rows)?;
                Ok(GroupIter { rows, columns }.collect())
            }
            _ => {
                // `NestedGroup` writes `self.depth - 1` levels (its own
                // depth); this reader's depth counts the group's own
                // instance level on top of those.
                let topology = buffer.read_topology(self.n, self.depth - 1)?;
                let rows = topology.total();
                let columns = columns(&self.elements, buffer, rows)?;
                topology.eat_compound_values(
                    self.depth - 1,
                    GroupIter { rows, columns },
                    PyroValue::List,
                )
            }
        }
    }
}

impl ValueReader for GroupOptionReader {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_value<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroValue<'a>, WireError> {
        match self.depth {
            0 => match buffer.read_nullable(self.n) {
                true => read_row(&self.elements, buffer),
                false => Ok(PyroValue::Null),
            },
            _ => Ok(PyroValue::List(self.read_values(buffer)?)),
        }
    }
}

impl ValueNestedReader for GroupOptionReader {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_values<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<Vec<PyroValue<'a>>, WireError> {
        match self.depth {
            // A bare nullable group is one value, not a column of them.
            0 => Err(WireError::InvalidInodes),
            1 => {
                let mask = buffer.read_group_mask(self.n)?;
                let columns = columns(&self.elements, buffer, mask.count_ones())?;
                Ok(GroupMaskedIter {
                    mask,
                    idx: 0,
                    columns,
                }
                .collect())
            }
            // Past the bare mask, `self.depth - 1` tiers with the mask
            // behind them.
            _ => {
                let (topology, mask) = buffer.read_group_option_nest(self.n, self.depth - 1)?;
                let columns = columns(&self.elements, buffer, mask.count_ones())?;
                let samples = GroupMaskedIter {
                    mask,
                    idx: 0,
                    columns,
                };
                topology.eat_compound_values(self.depth - 1, samples, PyroValue::List)
            }
        }
    }
}
