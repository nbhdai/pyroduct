//! [`WalReader`]: open from the footer, or recover by scanning; then read rows
//! into views lent from the pool.

use compio::fs::File;
use compio::io::{AsyncReadAt, AsyncReadAtExt};
use pyro_schema::SchemaParts;
use pyroformat::buf::{ALIGN, HEADER_SIZE, MAX_PAYLOAD, PyroParser, PyroView};

use crate::error::{Result, WalError};
use crate::format::{
    FILE_HEADER_SIZE, FileHeader, ROW_ALIGN, ROW_PREFIX_SIZE, align_up, crc32c, decode_footer,
    decode_row_prefix, footer_size, probe_row_header,
};
use crate::message::{ExactBuf, ViewFill};
use crate::pool::Pool;

/// How much of an unfinished WAL the recovery scan reads at a time.
const SCAN_CHUNK: usize = 1 << 20;

/// Where a row lives.
#[derive(Debug, Clone, Copy)]
struct RowIndex {
    /// Absolute, at the row prefix.
    offset: u64,
    /// Bytes to read: row prefix and message, plus up to 15 bytes of
    /// alignment padding when the length comes from footer offsets.
    span: u64,
}

/// What [`WalReader::open`] did when the footer was missing or invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recovery {
    /// Bytes of the data region that did not parse as rows and were skipped,
    /// in 16-byte steps — typically a row torn off at the end of the file.
    pub skipped: u64,
}

/// Reads a WAL's rows into a pool of reusable slots and hands them out as
/// [`PyroView`]s.
pub struct WalReader<F> {
    file: F,
    capacity: u32,
    schema_hash: u64,
    schema: Vec<u8>,
    rows: Vec<RowIndex>,
    recovery: Option<Recovery>,
    pool: Pool,
}

impl WalReader<File> {
    /// [`open`](Self::open) a compio file, taking its length from metadata.
    pub async fn open_file(file: File, slots: usize) -> Result<Self> {
        let len = file.metadata().await?.len();
        Self::open(file, len, slots).await
    }
}

impl<F: AsyncReadAt> WalReader<F> {
    /// Open a WAL of `len` bytes, reading rows into `slots` reusable buffers.
    ///
    /// The row index comes from the footer. When it is missing or does not
    /// verify, the data region is scanned for this WAL's rows instead, and
    /// [`recovery`](Self::recovery) says so.
    pub async fn open(file: F, len: u64, slots: usize) -> Result<Self> {
        if len < FILE_HEADER_SIZE as u64 {
            return Err(WalError::BadHeader("shorter than a WAL header"));
        }
        let fixed: [u8; FILE_HEADER_SIZE] = read_vec(&file, 0, FILE_HEADER_SIZE)
            .await?
            .try_into()
            .unwrap();
        let header = FileHeader::decode(&fixed)?;
        let data_start = header.data_start();
        if data_start > len {
            return Err(WalError::BadHeader(
                "schema table runs past the end of the file",
            ));
        }
        let schema = read_vec(&file, FILE_HEADER_SIZE as u64, header.schema_len as usize).await?;
        header.check_crc(&fixed, &schema)?;
        let parts = SchemaParts::decode(&schema).map_err(WalError::BadSchema)?;
        if pyro_schema::schema_hash(&parts.meta, &parts.inodes) != header.schema_hash {
            return Err(WalError::BadSchema(
                "schema table does not match schema_hash",
            ));
        }

        let (rows, recovery) = match read_footer(&file, &header, len).await? {
            Some(rows) => (rows, None),
            None => {
                let (rows, skipped) = scan(&file, &header, len).await?;
                (rows, Some(Recovery { skipped }))
            }
        };

        Ok(Self {
            file,
            capacity: header.capacity,
            schema_hash: header.schema_hash,
            schema,
            rows,
            recovery,
            pool: Pool::new(slots),
        })
    }

    /// Read row `row` into a view lent from the pool, waiting for a free slot
    /// if all are out, and verify it against the CRC in its row prefix.
    ///
    /// The view keeps the slot until its last clone drops, on any thread.
    /// [`PyroView::as_ref`] borrows a [`PyroRef`](pyroformat::buf::PyroRef)
    /// from it.
    pub async fn read(&self, row: usize) -> Result<PyroView> {
        let index = *self.rows.get(row).ok_or(WalError::OutOfRange {
            row,
            len: self.rows.len(),
        })?;
        let span = usize::try_from(index.span).map_err(|_| WalError::TooLarge)?;
        let corrupt = |reason| WalError::Corrupt { row, reason };
        // The row prefix goes to scratch, the message onto a lent view.
        let want = span
            .checked_sub(ROW_PREFIX_SIZE + HEADER_SIZE)
            .ok_or_else(|| corrupt("row shorter than a message header"))?;
        if want > MAX_PAYLOAD & !(ALIGN - 1) {
            return Err(WalError::TooLarge);
        }
        let view = self.pool.acquire(want).await;
        // SAFETY: freshly lent from the pool, so the view is its only handle.
        let fill = unsafe { ViewFill::new(view, HEADER_SIZE + want) };
        let buf = ([0; ROW_PREFIX_SIZE], (fill,));
        let compio::BufResult(res, (prefix, (fill,))) =
            self.file.read_vectored_exact_at(buf, index.offset).await;
        res?;

        let message = fill.bytes();
        let crc = decode_row_prefix(&prefix).ok_or_else(|| corrupt("bad row prefix"))?;
        PyroParser::check(message).map_err(|_| corrupt("bad message header"))?;
        let header: &[u8; HEADER_SIZE] = message[..HEADER_SIZE].try_into().unwrap();
        let len = probe_row_header(header, self.schema_hash)
            .ok_or_else(|| corrupt("not a data message of this WAL's schema"))?;
        let msg_len = HEADER_SIZE + len as usize;
        // What was read is the message plus at most its alignment padding.
        if msg_len > message.len() || message.len() - msg_len >= ROW_ALIGN as usize {
            return Err(corrupt("payload length disagrees with the footer"));
        }
        if crc32c(&message[..msg_len]) != crc {
            return Err(corrupt("CRC mismatch"));
        }
        Ok(fill.into_view())
    }

    /// Rows in the WAL.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The footer's row count, `N`.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// The hash every row's header carries.
    pub fn schema_hash(&self) -> u64 {
        self.schema_hash
    }

    /// The header's schema table: inode table and names.
    pub fn schema(&self) -> SchemaParts<'_> {
        SchemaParts::decode(&self.schema).expect("checked by open")
    }

    /// The schema table as stored, [`SchemaParts::encode`]d.
    pub fn schema_bytes(&self) -> &[u8] {
        &self.schema
    }

    /// `Some` when the footer was missing or invalid and the rows were found by
    /// scanning.
    pub fn recovery(&self) -> Option<Recovery> {
        self.recovery
    }

    /// The pool size, `K`.
    pub fn slots(&self) -> usize {
        self.pool.slots()
    }

    /// Slots not held by a read or a handed-out row.
    pub fn free_slots(&self) -> usize {
        self.pool.free()
    }

    pub fn get_ref(&self) -> &F {
        &self.file
    }

    /// Hand the file back. Rows already handed out stay valid.
    pub fn into_inner(self) -> F {
        self.file
    }
}

async fn read_vec<F: AsyncReadAt>(file: &F, offset: u64, len: usize) -> Result<Vec<u8>> {
    let compio::BufResult(res, buf) = file.read_exact_at(ExactBuf::new(len), offset).await;
    res?;
    Ok(buf.into_vec())
}

/// The row index from a valid footer, or `None` to recover instead.
async fn read_footer<F: AsyncReadAt>(
    file: &F,
    header: &FileHeader,
    len: u64,
) -> Result<Option<Vec<RowIndex>>> {
    let data_start = header.data_start();
    let size = footer_size(header.capacity);
    let Some(footer_start) = len.checked_sub(size).filter(|&s| s >= data_start) else {
        return Ok(None);
    };
    if !footer_start.is_multiple_of(ROW_ALIGN) {
        return Ok(None);
    }
    let footer = read_vec(file, footer_start, size as usize).await?;
    let Some(offsets) = decode_footer(header.capacity, &footer, footer_start - data_start) else {
        return Ok(None);
    };
    let starts = offsets.iter().map(|&o| data_start + o as u64);
    let ends = starts.clone().skip(1).chain([footer_start]);
    Ok(Some(
        starts
            .zip(ends)
            .map(|(offset, end)| RowIndex {
                offset,
                span: end - offset,
            })
            .collect(),
    ))
}

/// Recover the row index of a WAL without a footer: walk the data region in
/// 16-byte steps, looking for a row prefix followed by a message header
/// stamped with this WAL's schema hash, whose payload fits in the file and
/// matches the prefix's CRC. Anything else — a torn row, garbage — is skipped.
async fn scan<F: AsyncReadAt>(
    file: &F,
    header: &FileHeader,
    len: u64,
) -> Result<(Vec<RowIndex>, u64)> {
    const PROBE: u64 = (ROW_PREFIX_SIZE + HEADER_SIZE) as u64;
    let mut rows = Vec::new();
    let mut skipped = 0;
    let mut pos = header.data_start();
    let mut window_start = pos;
    let mut window: Vec<u8> = Vec::new();

    while pos + PROBE <= len && rows.len() < header.capacity as usize {
        if pos + PROBE > window_start + window.len() as u64 {
            window_start = pos;
            let want = (len - pos).min(SCAN_CHUNK as u64) as usize;
            window = read_vec(file, pos, want).await?;
        }
        let at = (pos - window_start) as usize;
        if let Some(span) = probe_row(file, header.schema_hash, &window, at, pos, len).await? {
            rows.push(RowIndex { offset: pos, span });
            pos = align_up(pos + span);
        } else {
            skipped += ROW_ALIGN;
            pos += ROW_ALIGN;
        }
    }
    // A tail shorter than a row prefix and header cannot hold a row.
    skipped += len.saturating_sub(pos.max(header.data_start()));
    Ok((rows, skipped))
}

/// The length of the row at `pos` (`window[at..]`), prefix and message, if
/// one is there intact.
async fn probe_row<F: AsyncReadAt>(
    file: &F,
    schema_hash: u64,
    window: &[u8],
    at: usize,
    pos: u64,
    len: u64,
) -> Result<Option<u64>> {
    let Some(crc) = decode_row_prefix(&window[at..]) else {
        return Ok(None);
    };
    let header_at = at + ROW_PREFIX_SIZE;
    let row_header: &[u8; HEADER_SIZE] = window[header_at..header_at + HEADER_SIZE]
        .try_into()
        .unwrap();
    let Some(payload) = probe_row_header(row_header, schema_hash) else {
        return Ok(None);
    };
    let msg_len = HEADER_SIZE + payload as usize;
    let span = (ROW_PREFIX_SIZE + msg_len) as u64;
    if pos + span > len {
        return Ok(None);
    }
    let found = if header_at + msg_len <= window.len() {
        crc32c(&window[header_at..header_at + msg_len])
    } else {
        crc32c(&read_vec(file, pos + ROW_PREFIX_SIZE as u64, msg_len).await?)
    };
    Ok((found == crc).then_some(span))
}
