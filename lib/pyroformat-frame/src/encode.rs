//! Encoding a Vortex file into ordered buffers without performing any IO.
//!
//! This follows `vortex_file::VortexWriteOptions::write`, built only on the public `vortex-file`
//! and `vortex-layout` APIs, so the bytes it yields can be written by any IO stack. User metadata
//! segments are not supported: their serializer hooks are private to `vortex-file`.
//!
//! Adapted from `vortex-file` 0.88.0, Copyright the Vortex contributors, Apache-2.0.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use futures::SinkExt;
use futures::StreamExt;
use futures::TryStreamExt;
use futures::channel::mpsc;
use futures::future::ready;
use vortex_array::ArrayContext;
use vortex_array::ArrayId;
use vortex_array::dtype::DType;
use vortex_array::expr::stats::Stat;
use vortex_array::session::ArraySessionExt;
use vortex_array::stats::PRUNING_STATS;
use vortex_array::stream::ArrayStream;
use vortex_array::stream::ArrayStreamExt;
use vortex_array::stream::SendableArrayStream;
use vortex_btrblocks::BtrBlocksCompressorBuilder;
use vortex_buffer::Alignment;
use vortex_buffer::ByteBuffer;
use vortex_edition::ComponentKind;
use vortex_edition::EditionSessionExt;
use vortex_error::VortexError;
use vortex_error::VortexExpect;
use vortex_error::VortexResult;
use vortex_error::vortex_bail;
use vortex_error::vortex_err;
use vortex_file::FileStatistics;
use vortex_file::Footer;
use vortex_file::MAGIC_BYTES;
use vortex_file::SegmentSpec;
use vortex_file::WriteStrategyBuilder;
use vortex_io::runtime::Task;
use vortex_io::session::RuntimeSessionExt;
use vortex_layout::LayoutContext;
use vortex_layout::LayoutRef;
use vortex_layout::LayoutStrategy;
use vortex_layout::LayoutWriterContext;
use vortex_layout::layouts::file_stats::FileStatsAccumulator;
use vortex_layout::layouts::file_stats::accumulate_stats;
use vortex_layout::segments::SegmentId;
use vortex_layout::segments::SegmentSink;
use vortex_layout::sequence::SequenceId;
use vortex_layout::sequence::SequentialStreamAdapter;
use vortex_layout::sequence::SequentialStreamExt;
use vortex_session::VortexSession;
use vortex_session::registry::Id;
use vortex_session::registry::ReadContext;
use vortex_utils::aliases::hash_set::HashSet;

/// Configures how a Vortex file is encoded, mirroring `vortex_file::VortexWriteOptions`.
///
/// Serialized arrays, layouts, extension dtypes, and zone-map aggregates are restricted to the
/// session's enabled editions unless edition enforcement is disabled.
pub struct EncodeOptions {
    session: VortexSession,
    strategy: Option<Arc<dyn LayoutStrategy>>,
    disable_editions: bool,
    exclude_dtype: bool,
    max_variable_length_statistics_size: usize,
    file_statistics: Vec<Stat>,
}

impl EncodeOptions {
    /// Options using `session`'s encodings, editions, and CPU runtime.
    pub fn new(session: VortexSession) -> Self {
        Self {
            session,
            strategy: None,
            disable_editions: false,
            exclude_dtype: false,
            max_variable_length_statistics_size: 64,
            file_statistics: PRUNING_STATS.to_vec(),
        }
    }

    /// Replace the default layout strategy.
    pub fn with_strategy(mut self, strategy: Arc<dyn LayoutStrategy>) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Disable all edition enforcement, permitting every registered encoding.
    pub fn disable_editions(mut self) -> Self {
        self.disable_editions = true;
        self
    }

    /// Exclude the dtype from the file. Readers must then supply it.
    pub fn exclude_dtype(mut self) -> Self {
        self.exclude_dtype = true;
        self
    }

    /// The statistics to compute at the file level. Pass an empty vector to omit them.
    pub fn with_file_statistics(mut self, file_statistics: Vec<Stat>) -> Self {
        self.file_statistics = file_statistics;
        self
    }

    /// Starts encoding `stream`. Layout and compression run on the session's runtime, which must
    /// be driven by threads of its own, such as a [`crate::CpuRuntime`].
    pub fn encode<S: ArrayStream + Send + 'static>(self, stream: S) -> VortexResult<FileEncoder> {
        let stream: SendableArrayStream = ArrayStreamExt::boxed(stream);
        let enforce_editions = !self.disable_editions;

        let ctx = LayoutWriterContext::new(new_array_context(&self.session, enforce_editions));
        let ctx = if enforce_editions {
            ctx.with_allowed_aggregates(edition_filter(&self.session, ComponentKind::Aggregate))
        } else {
            ctx
        };
        let strategy = match self.strategy {
            Some(strategy) => strategy,
            None => {
                let mut compressor = BtrBlocksCompressorBuilder::from_session(&self.session);
                if !enforce_editions {
                    compressor = compressor.disable_editions();
                }
                WriteStrategyBuilder::from_session(&self.session)
                    .with_btrblocks_builder(compressor)
                    .build()
            }
        };
        let dtype = stream.dtype().clone();
        if enforce_editions {
            validate_dtype_editions(&self.session, &dtype)?;
        }

        let (mut ptr, eof) = SequenceId::root().split();
        let stream = SequentialStreamAdapter::new(
            dtype.clone(),
            stream
                .try_filter(|chunk| ready(!chunk.is_empty()))
                .map(move |result| result.map(|chunk| (ptr.advance(), chunk))),
        )
        .sendable();
        let (file_stats, stream) = accumulate_stats(
            stream,
            self.file_statistics.clone().into(),
            self.max_variable_length_statistics_size,
            &self.session,
        );

        // Segment offsets start after the magic bytes, which the encoder yields first.
        let (send, recv) = mpsc::channel(1);
        let segments = Arc::new(ChannelSegmentSink::new(send, MAGIC_BYTES.len() as u64));

        let ctx2 = ctx.clone();
        let session = self.session.clone();
        let layout = self.session.handle().spawn_nested(move |_| async move {
            let layout = strategy
                .write_stream(
                    ctx2,
                    Arc::<ChannelSegmentSink>::clone(&segments),
                    stream,
                    eof,
                    &session,
                )
                .await?;
            Ok::<_, VortexError>((layout, segments.segment_specs()))
        });

        Ok(FileEncoder {
            phase: Phase::Magic,
            position: 0,
            segments: recv,
            layout: Some(layout),
            footer: FooterInputs {
                session: self.session,
                ctx,
                dtype,
                file_stats: (!self.file_statistics.is_empty()).then_some(file_stats),
                exclude_dtype: self.exclude_dtype,
                enforce_editions,
            },
            footer_buffers: VecDeque::new(),
            summary: None,
        })
    }
}

/// The result of encoding a file.
#[derive(Clone)]
pub struct EncodeSummary {
    footer: Footer,
    size: u64,
}

impl EncodeSummary {
    /// The footer of the encoded file.
    pub fn footer(&self) -> &Footer {
        &self.footer
    }

    /// The total size of the encoded file in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The number of rows in the encoded file.
    pub fn row_count(&self) -> u64 {
        self.footer.row_count()
    }
}

/// Produces the bytes of a Vortex file in order.
///
/// Call [`next_buffer`](Self::next_buffer) until it returns `None`, appending each buffer at the
/// [`position`](Self::position) read before the call, then call [`finish`](Self::finish).
pub struct FileEncoder {
    phase: Phase,
    position: u64,
    segments: mpsc::Receiver<ByteBuffer>,
    layout: Option<LayoutTask>,
    footer: FooterInputs,
    footer_buffers: VecDeque<ByteBuffer>,
    summary: Option<EncodeSummary>,
}

/// The background task that writes the layout, returning it with the segments it emitted.
type LayoutTask = Task<VortexResult<(LayoutRef, Arc<[SegmentSpec]>)>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Magic,
    Segments,
    Footer,
    Done,
}

/// Everything the footer needs that is fixed when encoding starts.
struct FooterInputs {
    session: VortexSession,
    ctx: LayoutWriterContext,
    dtype: DType,
    file_stats: Option<FileStatsAccumulator>,
    exclude_dtype: bool,
    enforce_editions: bool,
}

impl FileEncoder {
    /// The number of bytes yielded so far, which is the file offset of the next buffer.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// The next buffer of the file, or `None` once the footer has been yielded.
    ///
    /// Segment buffers are yielded as soon as the layout strategy emits them. Once the segment
    /// stream ends, the layout is awaited and the footer buffers follow.
    pub async fn next_buffer(&mut self) -> VortexResult<Option<ByteBuffer>> {
        loop {
            match self.phase {
                Phase::Magic => {
                    self.phase = Phase::Segments;
                    return Ok(Some(self.advance(ByteBuffer::copy_from(MAGIC_BYTES))));
                }
                Phase::Segments => match self.segments.next().await {
                    Some(buffer) if buffer.is_empty() => {}
                    Some(buffer) => return Ok(Some(self.advance(buffer))),
                    // Every sender is dropped once the layout task finishes.
                    None => {
                        self.encode_footer().await?;
                        self.phase = Phase::Footer;
                    }
                },
                Phase::Footer => match self.footer_buffers.pop_front() {
                    Some(buffer) => return Ok(Some(self.advance(buffer))),
                    None => self.phase = Phase::Done,
                },
                Phase::Done => return Ok(None),
            }
        }
    }

    /// The summary of the encoded file. Fails if [`next_buffer`](Self::next_buffer) has not yet
    /// returned `None`.
    pub fn finish(self) -> VortexResult<EncodeSummary> {
        match (self.phase, self.summary) {
            (Phase::Done, Some(summary)) => Ok(summary),
            _ => vortex_bail!("FileEncoder::finish called before every buffer was yielded"),
        }
    }

    fn advance(&mut self, buffer: ByteBuffer) -> ByteBuffer {
        self.position += buffer.len() as u64;
        buffer
    }

    async fn encode_footer(&mut self) -> VortexResult<()> {
        let layout = self
            .layout
            .take()
            .vortex_expect("the layout task is awaited once");
        let (layout, segment_specs) = layout.await?;
        let inputs = &self.footer;

        let statistics = inputs.file_stats.as_ref().map(|file_stats| {
            FileStatistics::new_with_dtype(file_stats.stats_sets().into(), &inputs.dtype)
        });
        let footer = Footer::new(
            layout,
            segment_specs,
            statistics,
            ReadContext::new(inputs.ctx.array_ctx().to_ids()),
        );

        let footer_offset = self.position;
        let footer_buffers = footer
            .clone()
            .into_serializer()
            .with_layout_context(new_layout_context(&inputs.session, inputs.enforce_editions))
            .with_offset(footer_offset)
            .with_exclude_dtype(inputs.exclude_dtype)
            .serialize()?;

        let size = footer_offset + footer_buffers.iter().map(|b| b.len() as u64).sum::<u64>();
        self.footer_buffers = footer_buffers.into();
        self.summary = Some(EncodeSummary { footer, size });
        Ok(())
    }
}

/// Assigns each segment its file offset and forwards its buffers, with alignment padding, to the
/// encoder in segment order.
struct ChannelSegmentSink {
    buffers: mpsc::Sender<ByteBuffer>,
    byte_offset: AtomicU64,
    segment_specs: Mutex<Vec<SegmentSpec>>,
}

impl ChannelSegmentSink {
    fn new(buffers: mpsc::Sender<ByteBuffer>, byte_offset: u64) -> Self {
        Self {
            buffers,
            byte_offset: AtomicU64::new(byte_offset),
            segment_specs: Mutex::default(),
        }
    }

    fn segment_specs(&self) -> Arc<[SegmentSpec]> {
        self.lock_specs().clone().into()
    }

    fn lock_specs(&self) -> std::sync::MutexGuard<'_, Vec<SegmentSpec>> {
        self.segment_specs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[async_trait]
impl SegmentSink for ChannelSegmentSink {
    async fn write(
        &self,
        mut sequence_id: SequenceId,
        buffers: Vec<ByteBuffer>,
    ) -> VortexResult<SegmentId> {
        // Waiting for every earlier sequence id gives this write exclusive, ordered access to the
        // byte offset while `sequence_id` is held.
        sequence_id.collapse().await;

        let (segment_id, padding) = {
            let mut specs = self.lock_specs();
            let segment_id = SegmentId::from(
                u32::try_from(specs.len()).map_err(|_| vortex_err!("Too many segments"))?,
            );
            // Segment buffers are written contiguously, so only the first one's alignment holds.
            let alignment = buffers
                .first()
                .map(|buffer| buffer.alignment())
                .unwrap_or_else(Alignment::none);
            let length = u32::try_from(buffers.iter().map(|buffer| buffer.len()).sum::<usize>())
                .map_err(|_| vortex_err!("Segment length exceeds u32"))?;

            let byte_offset = self.byte_offset.load(Ordering::Relaxed);
            let padding = byte_offset.next_multiple_of(alignment.as_usize() as u64) - byte_offset;
            let offset = byte_offset + padding;
            specs.push(SegmentSpec {
                offset,
                length,
                alignment,
            });
            self.byte_offset
                .store(offset + u64::from(length), Ordering::Relaxed);
            (segment_id, padding)
        };

        // A send fails only once the encoder is dropped, when the write is abandoned anyway.
        let mut send = self.buffers.clone();
        if padding > 0 {
            let _ = send.send(ByteBuffer::zeroed(padding as usize)).await;
        }
        for buffer in buffers {
            let _ = send.send(buffer).await;
        }
        Ok(segment_id)
    }
}

/// An array context seeded with every serialized id the file may contain, so encodings intern
/// in a deterministic order however compression is parallelized.
fn new_array_context(session: &VortexSession, enforce_editions: bool) -> ArrayContext {
    let registered: HashSet<ArrayId> = session
        .arrays()
        .registry()
        .read(|registry| registry.keys().copied().collect());
    let mut serialized_ids: Vec<ArrayId> = if enforce_editions {
        session
            .enabled_component_ids(ComponentKind::Array)
            .into_iter()
            .filter(|id| registered.contains(id))
            .collect()
    } else {
        registered.into_iter().collect()
    };
    serialized_ids.sort_unstable();
    let array_ctx = ArrayContext::new(serialized_ids.clone());
    if enforce_editions {
        array_ctx.with_allowed_ids(serialized_ids.into_iter().collect())
    } else {
        array_ctx
    }
}

fn edition_filter(session: &VortexSession, kind: ComponentKind) -> HashSet<Id> {
    session.enabled_component_ids(kind).into_iter().collect()
}

fn new_layout_context(session: &VortexSession, enforce_editions: bool) -> LayoutContext {
    let context = LayoutContext::default();
    if enforce_editions {
        context.with_allowed_ids(edition_filter(session, ComponentKind::Layout))
    } else {
        context
    }
}

/// Rejects extension dtypes nested anywhere in `dtype` that the enabled editions do not permit.
fn validate_dtype_editions(session: &VortexSession, dtype: &DType) -> VortexResult<()> {
    fn validate(dtype: &DType, allowed: &HashSet<Id>) -> VortexResult<()> {
        match dtype {
            DType::List(element, _) | DType::FixedSizeList(element, ..) => {
                validate(element, allowed)
            }
            DType::Map(map, _) => {
                validate(&map.key_dtype(), allowed)?;
                validate(&map.value_dtype(), allowed)
            }
            DType::Struct(fields, _) => fields
                .fields()
                .try_for_each(|field| validate(&field, allowed)),
            DType::Union(variants, _) => variants
                .variants()
                .try_for_each(|variant| validate(&variant, allowed)),
            DType::Extension(extension) => {
                if !allowed.contains(&extension.id()) {
                    vortex_bail!(
                        "Extension DType {} not permitted by enabled editions",
                        extension.id()
                    );
                }
                validate(extension.storage_dtype(), allowed)
            }
            DType::Null
            | DType::Bool(_)
            | DType::Primitive(..)
            | DType::Decimal(..)
            | DType::Utf8(_)
            | DType::Binary(_)
            | DType::Variant(_) => Ok(()),
        }
    }

    validate(dtype, &edition_filter(session, ComponentKind::DType))
}
