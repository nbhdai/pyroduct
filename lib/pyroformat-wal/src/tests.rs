use std::path::{Path, PathBuf};
use std::time::Duration;

use compio::fs::File;
use pyroformat::buf::{PyroCart, PyroHeader};
use pyroformat::{Layout, Reader, Realize, Writer};

use crate::format::{FileHeader, align_up};
use crate::{FILE_HEADER_SIZE, ROW_PREFIX_SIZE, WalError, WalReader, WalWriter, footer_size};

/// A row in the log.
#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
struct Event {
    /// Sequence number.
    seq: u64,
    kind: String,
    values: Vec<i32>,
}

/// A different schema, for rejection tests.
#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
struct Other {
    x: u32,
}

fn event(seq: u64) -> Event {
    Event {
        seq,
        kind: format!("event-{seq}"),
        values: (0..seq as i32 % 7).collect(),
    }
}

fn realize(view: pyroformat::buf::PyroView) -> Event {
    let cart = PyroCart::<Event>::from_view(view).unwrap();
    Realize::<Event>::realize(cart.get())
}

fn path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("events.wal")
}

/// Write `rows` events to a fresh WAL of `capacity`; finish it if asked.
async fn write_wal(path: &Path, capacity: u32, rows: u64, finish: bool) {
    let file = File::create(path).await.unwrap();
    let mut wal = WalWriter::create_for::<Event>(file, capacity)
        .await
        .unwrap();
    for seq in 0..rows {
        assert_eq!(wal.append_value(&event(seq)).await.unwrap(), seq as u32);
    }
    if finish {
        wal.finish().await.unwrap();
    }
}

async fn open(path: &Path, slots: usize) -> WalReader<File> {
    WalReader::open_file(File::open(path).await.unwrap(), slots)
        .await
        .unwrap()
}

async fn read_all(wal: &WalReader<File>) -> Vec<Event> {
    let mut out = Vec::new();
    for row in 0..wal.len() {
        out.push(realize(wal.read(row).await.unwrap()));
    }
    out
}

fn expected(seqs: impl IntoIterator<Item = u64>) -> Vec<Event> {
    seqs.into_iter().map(event).collect()
}

#[compio::test]
async fn finished_wal_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 16, 10, true).await;

    let wal = open(&path(&dir), 2).await;
    assert_eq!(wal.recovery(), None);
    assert_eq!(wal.len(), 10);
    assert_eq!(wal.capacity(), 16);
    assert_eq!(wal.schema_hash(), Event::SCHEMA_HASH);
    assert_eq!(read_all(&wal).await, expected(0..10));
    assert_eq!(wal.free_slots(), 2, "every slot came back");

    let schema = wal.schema();
    assert_eq!(schema.inodes, Event::INODES);
    assert_eq!(schema.field, Event::FLAT_NAMES);
    assert_eq!(schema.documentation[0], "Sequence number.");
}

#[compio::test]
async fn file_ends_in_a_fixed_footer() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 5, 3, true).await;
    let bytes = std::fs::read(path(&dir)).unwrap();

    let fixed: [u8; FILE_HEADER_SIZE] = bytes[..FILE_HEADER_SIZE].try_into().unwrap();
    let header = FileHeader::decode(&fixed).unwrap();
    assert_eq!(header.capacity, 5);
    assert_eq!(header.schema_hash, Event::SCHEMA_HASH);
    let data_start = header.data_start() as usize;
    assert_eq!(data_start % 16, 0);

    let footer = &bytes[bytes.len() - footer_size(5) as usize..];
    assert_eq!(footer.len(), 5 * 4 + 16);
    assert_eq!(&footer[footer.len() - 16..][..8], b"PYROWALF");
    assert_eq!(
        u32::from_le_bytes(footer[footer.len() - 8..][..4].try_into().unwrap()),
        3
    );
    // Row 0 sits at data_start; the unused rows are zero.
    assert_eq!(u32::from_le_bytes(footer[..4].try_into().unwrap()), 0);
    assert!(footer[3 * 4..5 * 4].iter().all(|&b| b == 0));
    // Each row is a CRC prefix, then a message stamped with the WAL's hash.
    let msg = data_start + ROW_PREFIX_SIZE;
    let row0: &[u8; 48] = bytes[msg..msg + 48].try_into().unwrap();
    assert_eq!(row0.schema_hash(), Event::SCHEMA_HASH);
    let len = 48 + row0.payload_len() as usize;
    let crc = u32::from_le_bytes(bytes[data_start..data_start + 4].try_into().unwrap());
    assert_eq!(crc, crate::format::crc32c(&bytes[msg..msg + len]));
    assert!(bytes[data_start + 4..msg].iter().all(|&b| b == 0));
}

#[compio::test]
async fn views_hold_slots_until_dropped_on_another_thread() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 8, 4, true).await;
    let wal = open(&path(&dir), 2).await;

    let a = wal.read(0).await.unwrap();
    let b = wal.read(1).await.unwrap();
    assert_eq!(wal.free_slots(), 0);

    // Both slots are out: the next read waits until a worker drops its row.
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        realize(a).seq
    });
    let c = wal.read(2).await.unwrap();
    assert_eq!(worker.join().unwrap(), 0);
    assert_eq!(realize(c), event(2));
    assert_eq!(realize(b), event(1));
    assert_eq!(wal.free_slots(), 2);
}

#[compio::test]
async fn rows_outlive_the_reader() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 8, 2, true).await;
    let wal = open(&path(&dir), 1).await;
    let view = wal.read(1).await.unwrap();
    drop(wal);
    assert_eq!(view.as_ref().schema_hash(), Event::SCHEMA_HASH);
    assert_eq!(realize(view), event(1));
}

#[compio::test]
async fn slots_grow_for_larger_rows() {
    let dir = tempfile::tempdir().unwrap();
    let file = File::create(path(&dir)).await.unwrap();
    let mut wal = WalWriter::create_for::<Event>(file, 4).await.unwrap();
    let small = event(1);
    let big = Event {
        seq: 2,
        kind: "x".repeat(10_000),
        values: (0..5_000).collect(),
    };
    wal.append_value(&small).await.unwrap();
    wal.append_value(&big).await.unwrap();
    wal.append_value(&small).await.unwrap();
    wal.finish().await.unwrap();

    let wal = open(&path(&dir), 1).await;
    assert_eq!(realize(wal.read(0).await.unwrap()), small);
    assert_eq!(realize(wal.read(1).await.unwrap()), big);
    assert_eq!(realize(wal.read(2).await.unwrap()), small);
}

#[compio::test]
async fn full_wal_rejects_appends() {
    let dir = tempfile::tempdir().unwrap();
    let file = File::create(path(&dir)).await.unwrap();
    let mut wal = WalWriter::create_for::<Event>(file, 2).await.unwrap();
    wal.append_value(&event(0)).await.unwrap();
    wal.append_value(&event(1)).await.unwrap();
    assert!(wal.is_full());
    assert!(matches!(
        wal.append_value(&event(2)).await,
        Err(WalError::Full { capacity: 2 })
    ));
    wal.finish().await.unwrap();
    assert_eq!(open(&path(&dir), 1).await.len(), 2);
}

// #[compio::test]
// async fn rejects_other_schemas_and_malformed_messages() {
//     let dir = tempfile::tempdir().unwrap();
//     let file = File::create(path(&dir)).await.unwrap();
//     let mut wal = WalWriter::create_for::<Event>(file, 4).await.unwrap();

//     assert!(matches!(
//         wal.append_value(&Other { x: 1 }).await,
//         Err(WalError::BadMessage(_))
//     ));
//     let other = Other { x: 1 }.encode().unwrap().view();
//     assert!(matches!(
//         wal.append_message(other).await,
//         Err(WalError::BadMessage(_))
//     ));
//     // A message with trailing bytes past its payload.
//     let mut bytes = event(0).encode().unwrap();
//     bytes(&[0; 16]);
//     let res = wal.append(bytes).await.0;
//     assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
//     // A plain Vec holding exactly a message is fine.
//     let bytes = event(0).encode().unwrap().view();
//     assert_eq!(wal.append(bytes).await.0.unwrap(), 0);
//     assert_eq!(wal.len(), 1);
// }

#[compio::test]
async fn unfinished_wal_is_recovered_by_scanning() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 16, 5, false).await;

    let wal = open(&path(&dir), 2).await;
    assert_eq!(wal.recovery().map(|r| r.skipped), Some(0));
    assert_eq!(read_all(&wal).await, expected(0..5));
}

#[compio::test]
async fn empty_unfinished_wal_has_no_rows() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 16, 0, false).await;
    let wal = open(&path(&dir), 1).await;
    assert!(wal.recovery().is_some());
    assert!(wal.is_empty());
}

#[compio::test]
async fn torn_tail_row_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 16, 5, false).await;
    let len = std::fs::metadata(path(&dir)).unwrap().len();
    // Cut the last row's payload short.
    std::fs::OpenOptions::new()
        .write(true)
        .open(path(&dir))
        .unwrap()
        .set_len(len - 3)
        .unwrap();

    let wal = open(&path(&dir), 2).await;
    assert!(wal.recovery().unwrap().skipped > 0);
    assert_eq!(read_all(&wal).await, expected(0..4));
}

#[compio::test]
async fn scan_resyncs_past_a_corrupt_row_header() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 16, 5, true).await;
    let ref_wal = open(&path(&dir), 1).await;
    let mut bytes = std::fs::read(path(&dir)).unwrap();
    // Find row 2 from a clean read, then smash its schema hash and drop the
    // footer so the reader has to scan.
    let fixed: [u8; FILE_HEADER_SIZE] = bytes[..FILE_HEADER_SIZE].try_into().unwrap();
    let data_start = FileHeader::decode(&fixed).unwrap().data_start() as usize;
    let mut row2 = data_start;
    for seq in 0..2 {
        let view = ref_wal.read(seq).await.unwrap();
        row2 = align_up((row2 + ROW_PREFIX_SIZE + view.as_message().len()) as u64) as usize;
    }
    bytes[row2 + ROW_PREFIX_SIZE + 8] ^= 0xff; // the message's schema hash
    bytes.truncate(bytes.len() - footer_size(16) as usize);
    std::fs::write(path(&dir), &bytes).unwrap();

    let wal = open(&path(&dir), 2).await;
    assert!(wal.recovery().unwrap().skipped > 0);
    assert_eq!(read_all(&wal).await, expected([0, 1, 3, 4]));
}

#[compio::test]
async fn recovery_drops_a_row_whose_payload_was_torn() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 16, 5, false).await;
    let mut bytes = std::fs::read(path(&dir)).unwrap();
    // The last row's header landed but its payload tail did not: the file is
    // full length, the bytes are zero.
    let n = bytes.len();
    bytes[n - 8..].fill(0);
    std::fs::write(path(&dir), &bytes).unwrap();

    let wal = open(&path(&dir), 2).await;
    assert!(wal.recovery().unwrap().skipped > 0);
    assert_eq!(read_all(&wal).await, expected(0..4));
}

#[compio::test]
async fn bad_footer_falls_back_to_recovery() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 8, 3, true).await;
    let mut bytes = std::fs::read(path(&dir)).unwrap();
    let n = bytes.len();
    bytes[n - 1] ^= 0xff; // footer CRC
    std::fs::write(path(&dir), &bytes).unwrap();

    let wal = open(&path(&dir), 1).await;
    assert!(wal.recovery().is_some());
    assert_eq!(read_all(&wal).await, expected(0..3));
}

#[compio::test]
async fn corrupt_payload_fails_its_crc() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 8, 3, true).await;
    let mut bytes = std::fs::read(path(&dir)).unwrap();
    let fixed: [u8; FILE_HEADER_SIZE] = bytes[..FILE_HEADER_SIZE].try_into().unwrap();
    let data_start = FileHeader::decode(&fixed).unwrap().data_start() as usize;
    bytes[data_start + ROW_PREFIX_SIZE + 60] ^= 0xff; // row 0's payload
    std::fs::write(path(&dir), &bytes).unwrap();

    let wal = open(&path(&dir), 1).await;
    assert!(matches!(
        wal.read(0).await,
        Err(WalError::Corrupt {
            row: 0,
            reason: "CRC mismatch"
        })
    ));
    assert_eq!(wal.free_slots(), 1, "a failed read returns its slot");
    assert_eq!(realize(wal.read(1).await.unwrap()), event(1));
}

#[compio::test]
async fn rejects_files_that_are_not_wals() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(path(&dir), [0u8; 64]).unwrap();
    let file = File::open(path(&dir)).await.unwrap();
    assert!(matches!(
        WalReader::open_file(file, 1).await,
        Err(WalError::BadHeader("bad magic"))
    ));

    write_wal(&path(&dir), 4, 1, true).await;
    let mut bytes = std::fs::read(path(&dir)).unwrap();
    bytes[FILE_HEADER_SIZE + 2] ^= 0xff; // inside the schema table
    std::fs::write(path(&dir), &bytes).unwrap();
    let file = File::open(path(&dir)).await.unwrap();
    assert!(matches!(
        WalReader::open_file(file, 1).await,
        Err(WalError::BadHeader("header CRC mismatch"))
    ));
}

#[compio::test]
async fn out_of_range_row() {
    let dir = tempfile::tempdir().unwrap();
    write_wal(&path(&dir), 4, 1, true).await;
    let wal = open(&path(&dir), 1).await;
    assert!(matches!(
        wal.read(1).await,
        Err(WalError::OutOfRange { row: 1, len: 1 })
    ));
}
