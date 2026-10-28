//! Opening Vortex files and reading their segments from a compio runtime.
//!
//! Opening reads and parses the footer. Reading then fetches every segment a [`Projection`]
//! needs in one burst of coalesced, concurrent reads, and returns a [`VortexFile`] over a
//! [`ResidentSegmentSource`], so scans and decoding never wait on IO.

use std::sync::Arc;

use compio::BufResult;
use futures::FutureExt;
use futures::StreamExt;
use futures::future;
use futures::stream;
use vortex_array::buffer::BufferHandle;
use vortex_array::dtype::DType;
use vortex_array::dtype::Field;
use vortex_array::dtype::FieldName;
use vortex_array::dtype::FieldPath;
use vortex_buffer::Alignment;
use vortex_buffer::ByteBuffer;
use vortex_error::VortexResult;
use vortex_error::vortex_bail;
use vortex_error::vortex_err;
use vortex_file::DeserializeStep;
use vortex_file::EOF_SIZE;
use vortex_file::Footer;
use vortex_file::MAX_POSTSCRIPT_SIZE;
use vortex_file::SegmentSpec;
use vortex_file::VortexFile;
use vortex_io::CoalesceConfig;
use vortex_layout::LayoutChildType;
use vortex_layout::LayoutRef;
use vortex_layout::segments::SegmentFuture;
use vortex_layout::segments::SegmentId;
use vortex_layout::segments::SegmentSource;
use vortex_session::VortexSession;
use vortex_utils::aliases::hash_map::HashMap;

use crate::CompioReadAt;
use crate::buf::ReadBuffer;

/// Options for opening a Vortex file from a [`CompioReadAt`] source.
pub struct CompioOpenOptions {
    session: VortexSession,
    file_size: Option<u64>,
    dtype: Option<DType>,
    initial_read_size: usize,
}

impl CompioOpenOptions {
    /// Options that open files with `session`'s registered encodings and runtime.
    pub fn new(session: VortexSession) -> Self {
        Self {
            session,
            file_size: None,
            dtype: None,
            initial_read_size: MAX_POSTSCRIPT_SIZE as usize + EOF_SIZE,
        }
    }

    /// The file size, if known, saving a size request.
    pub fn with_file_size(mut self, file_size: u64) -> Self {
        self.file_size = Some(file_size);
        self
    }

    /// The file's dtype, required when the file was written without one.
    pub fn with_dtype(mut self, dtype: DType) -> Self {
        self.dtype = Some(dtype);
        self
    }

    /// The number of trailing bytes to read first. Reading the whole footer at once saves a
    /// round trip; the default covers only the postscript.
    pub fn with_initial_read_size(mut self, initial_read_size: usize) -> Self {
        self.initial_read_size = initial_read_size;
        self
    }

    /// Reads and parses the footer of the file in `reader`.
    pub async fn open<R: CompioReadAt + ?Sized>(self, reader: &R) -> VortexResult<CompioFile> {
        let file_size = match self.file_size {
            Some(file_size) => file_size,
            None => reader.size().await?,
        };
        let mut initial_read_size = self
            .initial_read_size
            .max(MAX_POSTSCRIPT_SIZE as usize + EOF_SIZE);
        if let Ok(file_size) = usize::try_from(file_size) {
            initial_read_size = initial_read_size.min(file_size);
        }

        let initial_offset = file_size - initial_read_size as u64;
        let initial_read =
            read_exact(reader, initial_offset, initial_read_size, Alignment::none()).await?;
        let mut deserializer = Footer::deserializer(initial_read, self.session.clone())
            .with_size(file_size)
            .with_some_dtype(self.dtype);

        let footer = loop {
            match deserializer.deserialize()? {
                DeserializeStep::NeedMoreData { offset, len } => {
                    deserializer
                        .prefix_data(read_exact(reader, offset, len, Alignment::none()).await?);
                }
                DeserializeStep::NeedFileSize => {
                    vortex_bail!("Footer deserializer asked for the file size it was given")
                }
                DeserializeStep::Done(footer) => break footer,
            }
        };
        validate_file_size(&footer, file_size)?;

        Ok(CompioFile {
            footer,
            session: self.session,
        })
    }
}

/// An opened Vortex file whose segments have not been read.
#[derive(Clone)]
pub struct CompioFile {
    footer: Footer,
    session: VortexSession,
}

impl CompioFile {
    /// The parsed footer.
    pub fn footer(&self) -> &Footer {
        &self.footer
    }

    /// The file's dtype.
    pub fn dtype(&self) -> &DType {
        self.footer.dtype()
    }

    /// The file's row count.
    pub fn row_count(&self) -> u64 {
        self.footer.row_count()
    }

    /// The segments `projection` needs, sorted and without duplicates.
    pub fn segment_ids(&self, projection: &Projection) -> VortexResult<Vec<SegmentId>> {
        let selection = Selection::new(projection);
        let mut segment_ids = Vec::new();
        collect_segments(self.footer.layout(), &selection, &mut segment_ids)?;
        segment_ids.sort_unstable();
        segment_ids.dedup();
        Ok(segment_ids)
    }

    /// Reads every segment `projection` needs from `reader` and returns a [`VortexFile`] whose
    /// scans resolve those segments from memory.
    ///
    /// Scans of the returned file may only touch the projected fields, including any fields a
    /// filter refers to; requesting any other segment fails.
    pub async fn read<R: CompioReadAt + ?Sized>(
        &self,
        reader: &R,
        projection: &Projection,
    ) -> VortexResult<VortexFile> {
        let segment_map = self.footer.segment_map();
        let specs = self
            .segment_ids(projection)?
            .into_iter()
            .map(|id| {
                segment_map
                    .get(*id as usize)
                    .map(|spec| (id, *spec))
                    .ok_or_else(|| vortex_err!("Layout references missing segment {id}"))
            })
            .collect::<VortexResult<Vec<_>>>()?;

        let ranges = coalesce(specs, reader.coalesce_config());
        let mut reads = stream::iter(ranges.into_iter().map(|range| read_range(reader, range)))
            .buffer_unordered(reader.concurrency().max(1));
        let mut segments = HashMap::new();
        while let Some(read) = reads.next().await {
            segments.extend(read?);
        }

        Ok(VortexFile::new(
            self.footer.clone(),
            Arc::new(ResidentSegmentSource { segments }),
            self.session.clone(),
        ))
    }
}

/// Which fields of a file to read.
#[derive(Clone, Debug, Default)]
pub enum Projection {
    /// Every segment in the file.
    #[default]
    All,
    /// The segments of these fields, with everything nested beneath them, plus segments that
    /// are not specific to any field, such as file-wide zone maps.
    Fields(Vec<FieldPath>),
}

impl Projection {
    /// The named top-level fields.
    pub fn field_names<N: Into<FieldName>>(names: impl IntoIterator<Item = N>) -> Self {
        Self::Fields(names.into_iter().map(FieldPath::from_name).collect())
    }
}

/// [`Projection`] as a tree of selected field names.
#[derive(Default)]
struct Selection {
    all: bool,
    fields: HashMap<FieldName, Selection>,
}

impl Selection {
    fn new(projection: &Projection) -> Self {
        let mut root = Self::default();
        match projection {
            Projection::All => root.all = true,
            Projection::Fields(paths) => {
                for path in paths {
                    let mut node = &mut root;
                    for part in path.parts() {
                        // A list element shares its list's layout, so it selects nothing more.
                        if let Field::Name(name) = part {
                            node = node.fields.entry(name.clone()).or_default();
                        }
                    }
                    node.all = true;
                }
            }
        }
        root
    }
}

fn collect_segments(
    layout: &LayoutRef,
    selection: &Selection,
    segment_ids: &mut Vec<SegmentId>,
) -> VortexResult<()> {
    segment_ids.extend(layout.segment_ids());
    for slot in 0..layout.nslots() {
        let (Some(child_type), Some(child)) = (layout.slot_type(slot), layout.slot(slot)?) else {
            continue;
        };
        match child_type {
            LayoutChildType::Field(name) if !selection.all => {
                if let Some(selection) = selection.fields.get(&name) {
                    collect_segments(&child, selection, segment_ids)?;
                }
            }
            _ => collect_segments(&child, selection, segment_ids)?,
        }
    }
    Ok(())
}

/// One read covering one or more segments.
struct ReadRange {
    start: u64,
    end: u64,
    alignment: Alignment,
    segments: Vec<(SegmentId, SegmentSpec)>,
}

/// Merges segments into reads per `config`, with segments sorted by offset first.
fn coalesce(
    mut specs: Vec<(SegmentId, SegmentSpec)>,
    config: Option<CoalesceConfig>,
) -> Vec<ReadRange> {
    specs.sort_unstable_by_key(|(_, spec)| spec.offset);
    let mut ranges: Vec<ReadRange> = Vec::new();
    for (id, spec) in specs {
        let end = spec.offset + u64::from(spec.length);
        if let (Some(config), Some(range)) = (config, ranges.last_mut())
            && spec.offset <= range.end.saturating_add(config.distance)
            && end.saturating_sub(range.start) <= config.max_size
        {
            range.end = range.end.max(end);
            range.alignment = range.alignment.max(spec.alignment);
            range.segments.push((id, spec));
            continue;
        }
        ranges.push(ReadRange {
            start: spec.offset,
            end,
            alignment: spec.alignment,
            segments: vec![(id, spec)],
        });
    }
    ranges
}

/// Reads `range` into one buffer and slices out its segments.
///
/// The read starts at the range's start rounded down to its largest segment alignment, into a
/// buffer with that alignment, so segments the writer aligned within the file stay aligned in
/// memory without copying.
async fn read_range<R: CompioReadAt + ?Sized>(
    reader: &R,
    range: ReadRange,
) -> VortexResult<Vec<(SegmentId, BufferHandle)>> {
    let align = range.alignment.as_usize() as u64;
    let read_start = range.start - range.start % align;
    let len = usize::try_from(range.end - read_start)
        .map_err(|_| vortex_err!("Read of {} bytes exceeds usize", range.end - read_start))?;
    let buffer = read_exact(reader, read_start, len, range.alignment).await?;

    Ok(range
        .segments
        .into_iter()
        .map(|(id, spec)| {
            let begin = usize::try_from(spec.offset - read_start)
                .unwrap_or_else(|_| unreachable!("segment lies within a usize-sized read"));
            let segment = buffer
                .slice_unaligned(begin..begin + spec.length as usize)
                .aligned(spec.alignment);
            (id, BufferHandle::new_host(segment))
        })
        .collect())
}

/// Fails if any segment the footer declares lies beyond `file_size`.
fn validate_file_size(footer: &Footer, file_size: u64) -> VortexResult<()> {
    for (id, spec) in footer.segment_map().iter().enumerate() {
        let end = spec.offset.checked_add(u64::from(spec.length));
        if end.is_none_or(|end| end > file_size) {
            vortex_bail!(
                "Segment {id} at {}+{} exceeds the file size {file_size}",
                spec.offset,
                spec.length
            );
        }
    }
    Ok(())
}

async fn read_exact<R: CompioReadAt + ?Sized>(
    reader: &R,
    offset: u64,
    len: usize,
    alignment: Alignment,
) -> VortexResult<ByteBuffer> {
    let BufResult(result, buffer) = reader
        .read_exact_at(ReadBuffer::new(len, alignment), offset)
        .await;
    result?;
    if !buffer.is_full() {
        vortex_bail!("Short read of {len} bytes at offset {offset}");
    }
    Ok(buffer.freeze())
}

/// A [`SegmentSource`] over segments already in memory.
///
/// Requests resolve immediately, so scans over it never wait on IO.
pub struct ResidentSegmentSource {
    segments: HashMap<SegmentId, BufferHandle>,
}

impl ResidentSegmentSource {
    /// A source serving `segments`.
    pub fn new(segments: HashMap<SegmentId, BufferHandle>) -> Self {
        Self { segments }
    }
}

impl SegmentSource for ResidentSegmentSource {
    fn request(&self, id: SegmentId) -> SegmentFuture {
        future::ready(self.segments.get(&id).cloned().ok_or_else(|| {
            vortex_err!("Segment {id} was not read; add the fields that use it to the Projection")
        }))
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use vortex_buffer::Alignment;
    use vortex_file::SegmentSpec;
    use vortex_io::CoalesceConfig;
    use vortex_layout::segments::SegmentId;

    use super::coalesce;

    fn spec(offset: u64, length: u32, alignment: usize) -> SegmentSpec {
        SegmentSpec {
            offset,
            length,
            alignment: Alignment::new(alignment),
        }
    }

    #[rstest]
    #[case::none(None, 3)]
    #[case::adjacent(Some(CoalesceConfig::new(0, 1 << 20)), 2)]
    #[case::gaps(Some(CoalesceConfig::new(64, 1 << 20)), 1)]
    #[case::capped(Some(CoalesceConfig::new(64, 200)), 2)]
    fn coalesces_by_config(#[case] config: Option<CoalesceConfig>, #[case] expected: usize) {
        let specs = vec![
            (SegmentId::from(2), spec(200, 50, 8)),
            (SegmentId::from(0), spec(0, 64, 64)),
            (SegmentId::from(1), spec(64, 100, 8)),
        ];
        let ranges = coalesce(specs, config);
        assert_eq!(ranges.len(), expected);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].alignment, Alignment::new(64));
        assert_eq!(
            ranges.iter().map(|r| r.segments.len()).sum::<usize>(),
            3,
            "every segment lands in exactly one read"
        );
    }
}
