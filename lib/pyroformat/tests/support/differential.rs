//! The derive path against the value path on the same data, for
//! `tests/differential.rs` and `tests/corrupt_buffers.rs`.
//!
//! [`assert_agree`] recovers the data's value by reading the derive's bytes
//! with the value reader, re-encodes it with the value writer, and asserts
//! the two byte streams are equal and that each side reads its own bytes and
//! the other side's. Its failure message carries all of that, so it says
//! which side went wrong. Panics on either side are caught and reported
//! instead of ending the test with a bare backtrace.
#![allow(dead_code)]

use std::fmt::Debug;
use std::panic::{AssertUnwindSafe, catch_unwind};

use pyroformat::PyroValue;
use pyroformat::buf::PyroSink;
use pyroformat::buf::sink::LocalMemSink;
use pyroformat::schema::{PyroSchema, SchemaParts, schema_hash};
use pyroformat::wire::value_readers::RowReader;
use pyroformat::wire::value_writers::ValueWriter;
use pyroformat::{Layout, Reader, Realize, Writer};

/// `T`'s schema, realized from its `Layout` tables.
pub fn schema_of<T: Layout>() -> PyroSchema<'static> {
    SchemaParts {
        inodes: T::INODES.to_vec(),
        field: T::FLAT_NAMES.to_vec(),
        documentation: T::FLAT_DOCS.to_vec(),
        schema_documentation: None,
        meta: T::META,
    }
    .to_schema()
    .expect("a derived layout realizes")
}

/// The value writer's message for `value`, laid out by `T`'s inodes.
pub fn value_write<T: Layout>(value: &PyroValue) -> Result<Vec<u8>, String> {
    let schema = schema_of::<T>();
    let vw = ValueWriter::new(&schema).map_err(|e| format!("ValueWriter::new: {e:?}"))?;
    let vw = vw.ingest(value).map_err(|e| format!("ingest: {e:?}"))?;
    let mut sink = LocalMemSink::new(vw.fixed_len(), schema_hash(&T::META, T::INODES));
    sink.reallocate_sync(vw.variable_len())
        .map_err(|e| format!("{e:?}"))?;
    vw.write(&mut sink).map_err(|e| format!("write: {e:?}"))?;
    Ok(sink.payload().to_vec())
}

/// Runs `f`, turning a panic into `Err` with its message.
pub fn guard<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|p| {
        p.downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "non-string panic".into())
    })
}

/// What one reader made of a message.
#[derive(Debug, PartialEq)]
pub enum Read<V> {
    Accepted(V),
    Rejected(String),
    Panicked(String),
}

/// The derive reader on `bytes`, realized.
pub fn derive_read<T: Reader>(bytes: &[u8]) -> Read<T> {
    match guard(|| T::pyro_read(bytes).map(|r| r.realize())) {
        Ok(Ok(v)) => Read::Accepted(v),
        Ok(Err(e)) => Read::Rejected(format!("{e:?}")),
        Err(p) => Read::Panicked(p),
    }
}

/// The value reader on `bytes`, as a debug string (a `PyroValue` borrows
/// the bytes).
pub fn value_read<T: Layout>(bytes: &[u8]) -> Read<String> {
    let schema = schema_of::<T>();
    let reader = RowReader::new(&schema).expect("RowReader for a derived layout");
    match guard(|| reader.read_value(bytes).map(|v| format!("{v:?}"))) {
        Ok(Ok(v)) => Read::Accepted(v),
        Ok(Err(e)) => Read::Rejected(format!("{e:?}")),
        Err(p) => Read::Panicked(p),
    }
}

/// Asserts the derive and the value path agree on `data`: same bytes, and
/// every reader reads every writer's bytes back to `data`.
#[track_caller]
pub fn assert_agree<T: Writer + Reader + Layout + PartialEq + Debug>(data: &T) {
    if let Some(report) = disagreement(data) {
        panic!("derive and value path disagree on {data:?}\n{report}");
    }
}

/// Asserts both writers reject `data` with `Err` (neither panics nor writes
/// bytes). `wide` is the same data under a wider word: the derive's bytes
/// can't supply the value when the derive rejects, so the value comes from
/// `wide`'s.
#[track_caller]
pub fn assert_both_reject<T: Writer + Layout + Debug, Wide: Writer + Layout>(
    data: &T,
    wide: &Wide,
) {
    let derive =
        guard(|| data.encode().map(|b| b.payload().len())).map(|r| r.map_err(|e| format!("{e:?}")));
    let wide_bytes = wide
        .encode()
        .expect("the wide twin encodes")
        .payload()
        .to_vec();
    let schema = schema_of::<Wide>();
    let reader = RowReader::new(&schema).expect("RowReader for a derived layout");
    let value = reader
        .read_value(&wide_bytes)
        .expect("the value reader reads the wide twin");
    let written = guard(|| value_write::<T>(&value).map(|b| b.len()));
    assert!(
        matches!(derive, Ok(Err(_))) && matches!(written, Ok(Err(_))),
        "both writers must return Err on {data:?}\n  derive writer: {}\n  value writer: {}",
        outcome(&derive),
        outcome(&written),
    );
}

fn outcome(r: &Result<Result<usize, String>, String>) -> String {
    match r {
        Ok(Ok(len)) => format!("returned Ok ({len} bytes)"),
        Ok(Err(e)) => format!("returned Err: {e}"),
        Err(p) => format!("PANICKED: {p}"),
    }
}

fn disagreement<T: Writer + Reader + Layout + PartialEq + Debug>(data: &T) -> Option<String> {
    let derive = match guard(|| data.encode()) {
        Ok(Ok(b)) => b.payload().to_vec(),
        Ok(Err(e)) => return Some(format!("derive writer: returned Err({e:?})")),
        Err(p) => return Some(format!("derive writer: PANICKED: {p}")),
    };
    let derive_self = derive_read::<T>(&derive);

    let schema = schema_of::<T>();
    let reader = RowReader::new(&schema).expect("RowReader for a derived layout");
    let value = match guard(|| reader.read_value(&derive)) {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            return Some(format!(
                "value reader: REJECTS the derive's bytes: {e:?}\n\
                 derive reads its own bytes: {}\n\
                 derive bytes: {derive:?}",
                verdict(&derive_self, data),
            ));
        }
        Err(p) => {
            return Some(format!(
                "value reader: PANICKED on the derive's bytes: {p}\n\
                 derive reads its own bytes: {}\n\
                 derive bytes: {derive:?}",
                verdict(&derive_self, data),
            ));
        }
    };
    let golden = match guard(|| value_write::<T>(&value)) {
        Ok(Ok(g)) => g,
        Ok(Err(e)) => {
            return Some(format!(
                "value writer: returned Err on the value it read: {e}\n\
                 value: {value:?}\nderive bytes: {derive:?}"
            ));
        }
        Err(p) => {
            return Some(format!(
                "value writer: PANICKED on the value it read: {p}\n\
                 value: {value:?}\nderive bytes: {derive:?}"
            ));
        }
    };
    let value_self = guard(|| reader.read_value(&golden).map(|w| w == value));
    let derive_cross = derive_read::<T>(&golden);

    let bytes_agree = derive == golden;
    let derive_self_ok = matches!(&derive_self, Read::Accepted(v) if v == data);
    let value_self_ok = matches!(value_self, Ok(Ok(true)));
    let derive_cross_ok = matches!(&derive_cross, Read::Accepted(v) if v == data);
    if bytes_agree && derive_self_ok && value_self_ok && derive_cross_ok {
        return None;
    }
    Some(format!(
        "bytes agree: {bytes_agree}\n\
         derive reads its own bytes: {}\n\
         value path reads its own bytes back to the same value: {value_self_ok}\n\
         derive reads the value path's bytes: {}\n\
         value (read from the derive's bytes): {value:?}\n\
         derive bytes: {derive:?}\n\
         value bytes:  {golden:?}",
        verdict(&derive_self, data),
        verdict(&derive_cross, data),
    ))
}

fn verdict<T: PartialEq + Debug>(read: &Read<T>, data: &T) -> String {
    match read {
        Read::Accepted(v) if v == data => "ok".into(),
        Read::Accepted(v) => format!("WRONG, got {v:?}"),
        Read::Rejected(e) => format!("REJECTED: {e}"),
        Read::Panicked(p) => format!("PANICKED: {p}"),
    }
}
