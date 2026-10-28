//! [`WalWriter`]: header and schema first, then rows, then the footer.

use compio::BufResult;
use compio::buf::IoBuf;
use compio::io::{AsyncWriteAt, AsyncWriteAtExt};
use pyro_schema::SchemaParts;
use pyroformat::buf::{HEADER_SIZE, PyroBuf, PyroView};
use pyroformat::{Layout, Writer};

use crate::error::{Result, WalError};
use crate::format::{
    FileHeader, ROW_PREFIX_SIZE, align_up, crc32c, encode_footer, encode_row_prefix,
    probe_row_header,
};

/// Appends pyroformat messages of one schema to a WAL of at most `capacity`
/// rows.
///
/// The header and schema table are on disk once [`create`](Self::create)
/// returns; each row once its [`append`](Self::append) returns. The footer is
/// written only by [`finish`](Self::finish) — a writer dropped before then
/// leaves a WAL the reader recovers by scanning. Nothing is synced: reach the
/// file through [`get_ref`](Self::get_ref) to sync it.
pub struct WalWriter<F> {
    file: F,
    capacity: u32,
    schema_hash: u64,
    data_start: u64,
    /// Absolute end of the last row, before alignment padding.
    end: u64,
    /// Each row's offset from `data_start`.
    rows: Vec<u32>,
}

impl<F: AsyncWriteAt> WalWriter<F> {
    /// Start a WAL for messages matching `schema`, with room for `capacity`
    /// rows. `include_doc` packs the schema's docstrings into the header too.
    pub async fn create(
        mut file: F,
        schema: &SchemaParts<'_>,
        include_doc: bool,
        capacity: u32,
    ) -> Result<Self> {
        let schema_hash = pyro_schema::schema_hash(&schema.meta, &schema.inodes);
        let table = schema.encode(include_doc);
        let header = FileHeader::encode(capacity, schema_hash, &table)?;
        let data_start = header.len() as u64;
        file.write_all_at(header, 0).await.0?;
        Ok(Self {
            file,
            capacity,
            schema_hash,
            data_start,
            end: data_start,
            rows: Vec::with_capacity(capacity as usize),
        })
    }

    /// [`create`](Self::create) with `T`'s schema, docstrings included.
    pub async fn create_for<T: Layout>(file: F, capacity: u32) -> Result<Self> {
        let schema = SchemaParts {
            inodes: T::INODES.to_vec(),
            field: T::FLAT_NAMES.to_vec(),
            documentation: T::FLAT_DOCS.to_vec(),
            schema_documentation: (!T::DOC.is_empty()).then_some(T::DOC),
            meta: T::META,
        };
        Self::create(file, &schema, true, capacity).await
    }

    /// Append one whole message — 48-byte header, then payload — as the next
    /// row, returning its index. It must be a [`Data`] message stamped with
    /// this WAL's schema hash, and nothing past its payload. It lands behind a
    /// row prefix holding its CRC-32C.
    ///
    /// [`Data`]: pyroformat::buf::DataStatus::Data
    pub async fn append(&mut self, message: PyroView) -> BufResult<u32, PyroView> {
        match self.check_append(&message) {
            Ok((offset, crc)) => self.write_row(message, offset, crc).await,
            Err(err) => BufResult(Err(err.into_io()), message),
        }
    }

    /// [`append`](Self::append) a sealed buffer, or a view of one.
    pub async fn append_message(&mut self, message: PyroView) -> Result<u32> {
        let (offset, crc) = self.check_append(&message)?;
        Ok(self.write_row(message, offset, crc).await.0?)
    }

    /// Encode `value` and [`append`](Self::append) it.
    pub async fn append_value<T: Writer>(&mut self, value: &T) -> Result<u32> {
        if T::SCHEMA_HASH != self.schema_hash {
            return Err(WalError::BadMessage(
                "value's schema hash is not this WAL's",
            ));
        }
        let buf: PyroBuf = value
            .encode()
            .map_err(|_| WalError::BadMessage("value failed to encode"))?;
        self.append_message(buf.view()).await
    }

    /// Write the footer and hand the file back.
    pub async fn finish(mut self) -> Result<F> {
        let footer = encode_footer(self.capacity, &self.rows);
        // The gap from the last row up to the aligned footer reads back as
        // zeros: a positional write past the end of a file fills the hole.
        self.file.write_all_at(footer, align_up(self.end)).await.0?;
        Ok(self.file)
    }

    /// Rows appended so far.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn is_full(&self) -> bool {
        self.rows.len() as u32 >= self.capacity
    }

    /// The hash every row's header must carry.
    pub fn schema_hash(&self) -> u64 {
        self.schema_hash
    }

    pub fn get_ref(&self) -> &F {
        &self.file
    }

    pub fn get_mut(&mut self) -> &mut F {
        &mut self.file
    }

    /// Write the row prefix and the message behind it in one vectored write.
    async fn write_row<B: IoBuf>(
        &mut self,
        message: B,
        offset: u64,
        crc: u32,
    ) -> BufResult<u32, B> {
        let len = (ROW_PREFIX_SIZE + message.buf_len()) as u64;
        let row = (encode_row_prefix(crc), (message,));
        let BufResult(res, (_, (message,))) = self.file.write_vectored_all_at(row, offset).await;
        if let Err(err) = res {
            return BufResult(Err(err), message);
        }
        let index = self.rows.len() as u32;
        self.rows.push((offset - self.data_start) as u32);
        self.end = offset + len;
        BufResult(Ok(index), message)
    }

    /// Where the next row goes and its message's CRC, or why it cannot.
    fn check_append(&self, bytes: &PyroView) -> Result<(u64, u32)> {
        if self.is_full() {
            return Err(WalError::Full {
                capacity: self.capacity,
            });
        }
        let header: &[u8; HEADER_SIZE] =
            bytes
                .get(..HEADER_SIZE)
                .and_then(|h| h.try_into().ok())
                .ok_or(WalError::BadMessage("shorter than a message header"))?;
        let len = probe_row_header(header, self.schema_hash).ok_or(WalError::BadMessage(
            "not a data message of this WAL's schema",
        ))?;
        if HEADER_SIZE as u64 + len as u64 != bytes.len() as u64 {
            return Err(WalError::BadMessage(
                "buffer length does not match the header's payload length",
            ));
        }
        let offset = align_up(self.end);
        if offset - self.data_start > u32::MAX as u64 {
            return Err(WalError::TooLarge);
        }
        Ok((offset, crc32c(bytes)))
    }
}

impl WalError {
    /// For [`WalWriter::append`], whose `BufResult` carries an `io::Error`.
    fn into_io(self) -> std::io::Error {
        match self {
            WalError::Io(err) => err,
            other => std::io::Error::new(std::io::ErrorKind::InvalidInput, other),
        }
    }
}
