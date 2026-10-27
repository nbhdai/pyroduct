use pyro_schema::Bow;

use crate::{
    MAX_TIERS, PyroValue,
    errors::WireError,
    schema::PyroType,
    wire::value_readers::{Reader, SplitSource, ValueNestedReader, ValueReader},
};
use pyro_schema::StructureWord;

#[derive(Debug, Clone)]
pub struct MapReader {
    n: usize,
    depth: usize,
    key_reader: Reader,
    value_reader: Reader,
}

impl MapReader {
    /// The read-side twin of `wire::value_writers::map::Map::new`.
    ///
    /// `depth` uses the writer's own convention — `nesting + group_nest` —
    /// because a map's entries are its intrinsic level: a root `Map`
    /// (nesting 0, not group-nested) is `depth == 0` and stores nothing but
    /// a bare entry count in its slot. Every level above that is one
    /// topology level, the last of them counting each map's entries.
    ///
    /// `key_reader`/`value_reader` read the key and value columns: every
    /// entry of every map in the buffer, in order.
    pub fn new(n: usize, depth: usize, key_reader: Reader, value_reader: Reader) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        tracing::trace!(n, depth, "Creating MapReader");
        Self {
            n,
            depth,
            key_reader,
            value_reader,
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Mirrors `Map`'s `IoType::datatype` — `List`-wrapped once per level
    /// *above* the map's own entry level.
    pub fn datatype(&self) -> PyroType {
        let mut t = PyroType::Map {
            key: Bow::new(self.key_reader.datatype()),
            value: Bow::new(self.value_reader.datatype()),
        };
        for _ in 0..self.depth.saturating_sub(1) {
            t = PyroType::List(Bow::new(t));
        }
        t
    }

    /// The key and value columns, zipped into entries, checked to hold
    /// `expected` of each.
    fn entries<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
        expected: usize,
    ) -> Result<impl Iterator<Item = (PyroValue<'a>, PyroValue<'a>)> + use<'a, W>, WireError> {
        let keys = self.key_reader.read_values(buffer)?;
        let values = self.value_reader.read_values(buffer)?;
        for (index, have) in [keys.len(), values.len()].into_iter().enumerate() {
            if have != expected {
                return Err(WireError::LengthMismatch {
                    expected,
                    have,
                    index,
                });
            }
        }
        Ok(keys.into_iter().zip(values))
    }
}

impl ValueReader for MapReader {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_value<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroValue<'a>, WireError> {
        match self.depth {
            0 => {
                let count = buffer.read_count(self.n);
                let map = self.entries(buffer, count)?.collect::<Vec<_>>();
                Ok(PyroValue::Map(map.into()))
            }
            _ => Ok(PyroValue::List(self.read_values(buffer)?)),
        }
    }
}

impl ValueNestedReader for MapReader {
    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn read_values<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<Vec<PyroValue<'a>>, WireError> {
        // A bare map is one value, not a column of them.
        if self.depth == 0 {
            return Err(WireError::InvalidInodes);
        }
        // `depth` topology levels, the last counting entries per map.
        let topology = buffer.read_topology(self.n, self.depth)?;
        let entries = self.entries(buffer, topology.total())?;
        topology.eat_compound_values(self.depth, entries, |entries| {
            PyroValue::Map(entries.into())
        })
    }
}
