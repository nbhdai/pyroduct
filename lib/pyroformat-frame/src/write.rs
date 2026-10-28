//! Writing Vortex files from a compio runtime.
//!
//! Both writers drive a [`FileEncoder`]: compression runs on the session's CPU runtime, and only
//! the finished buffers reach the compio thread, which owns every write.

use std::future::poll_fn;
use std::pin::pin;
use std::task::Poll;

use compio::io::AsyncWrite;
use compio::io::AsyncWriteExt;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use vortex_array::stream::ArrayStream;
use vortex_buffer::ByteBuffer;
use vortex_error::VortexResult;

use crate::CompioWriteAt;
use crate::EncodeOptions;
use crate::EncodeSummary;
use crate::FileEncoder;
use crate::buf::WriteBuffer;

/// Writes `stream` as a Vortex file to a sequential sink such as a socket, one buffer at a time.
///
/// The sink is flushed once the footer is written.
pub async fn write<W, S>(
    options: EncodeOptions,
    sink: &mut W,
    stream: S,
) -> VortexResult<EncodeSummary>
where
    W: AsyncWrite + ?Sized,
    S: ArrayStream + Send + 'static,
{
    let mut encoder = options.encode(stream)?;
    while let Some(buffer) = encoder.next_buffer().await? {
        sink.write_all(WriteBuffer(buffer)).await.0?;
    }
    sink.flush().await?;
    encoder.finish()
}

/// Writes `stream` as a Vortex file to a positional sink, keeping up to
/// [`CompioWriteAt::max_in_flight`] writes in flight while the encoder produces the next buffer.
///
/// Every buffer's offset is known when the encoder yields it, so writes may complete in any
/// order. Nothing is synced to stable storage; call `sync_all` on the file for durability.
pub async fn write_at<W, S>(
    options: EncodeOptions,
    sink: &W,
    stream: S,
) -> VortexResult<EncodeSummary>
where
    W: CompioWriteAt + ?Sized,
    S: ArrayStream + Send + 'static,
{
    let mut encoder = options.encode(stream)?;
    let max_in_flight = sink.max_in_flight().max(1);
    let mut in_flight = FuturesUnordered::new();

    while let Some((offset, buffer)) = next_while_writing(&mut encoder, &mut in_flight).await? {
        in_flight.push(write_buffer(sink, buffer, offset));
        while in_flight.len() >= max_in_flight {
            if let Some(result) = in_flight.next().await {
                result?;
            }
        }
    }
    while let Some(result) = in_flight.next().await {
        result?;
    }
    encoder.finish()
}

/// The encoder's next buffer and its offset, polling in-flight writes while it is produced.
///
/// compio submits an operation when its future is first polled, so writes must be polled while
/// the encoder waits on compression, or they would sit unsubmitted until it yields.
async fn next_while_writing<F>(
    encoder: &mut FileEncoder,
    in_flight: &mut FuturesUnordered<F>,
) -> VortexResult<Option<(u64, ByteBuffer)>>
where
    F: Future<Output = std::io::Result<()>>,
{
    let offset = encoder.position();
    let mut next = pin!(encoder.next_buffer());
    let buffer = poll_fn(|cx| {
        while let Poll::Ready(Some(result)) = in_flight.poll_next_unpin(cx) {
            if let Err(err) = result {
                return Poll::Ready(Err(err.into()));
            }
        }
        next.as_mut().poll(cx)
    })
    .await?;
    Ok(buffer.map(|buffer| (offset, buffer)))
}

async fn write_buffer<W: CompioWriteAt + ?Sized>(
    sink: &W,
    buffer: ByteBuffer,
    offset: u64,
) -> std::io::Result<()> {
    sink.write_all_at(WriteBuffer(buffer), offset).await.0
}
