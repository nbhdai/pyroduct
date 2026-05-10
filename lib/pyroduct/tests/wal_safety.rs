use pyroduct::format::header::PyroData;
use pyroduct::format::wal::{WalReader, WalRecord, WalWriter};
use pyroduct::format::{Bridgeable, PyroFailure, PyroLogs, PyroRow, PyroSuccess, PyroValue, PyroView};
use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use std::thread;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_success_record(row_index: usize) -> WalRecord {
    WalRecord::Success {
        row_index,
        success: PyroSuccess {
            row: PyroRow::from([
                ("id", PyroValue::from(row_index as i32)),
                ("name", PyroValue::from("test")),
            ])
            .into_owned(),
            logs: PyroLogs {
                module_logs: vec![format!("processing row {}", row_index)],
                capability_logs: HashMap::new(),
            },
        },
    }
}

fn make_failure_record(row_index: usize) -> WalRecord {
    WalRecord::Failure {
        row_index,
        failure: PyroFailure {
            result: Err(format!("row {} failed", row_index)),
            logs: PyroLogs::empty(),
        },
    }
}

fn make_mixed_records(count: usize) -> Vec<WalRecord> {
    (0..count)
        .map(|i| {
            if i % 3 == 0 {
                make_failure_record(i)
            } else {
                make_success_record(i)
            }
        })
        .collect()
}

/// Helper to write records to memory buffers and return a WalReader.
fn write_and_open_in_memory(records: &[WalRecord]) -> WalReader {
    let mut wal_buf = Vec::new();
    let mut log_buf = Vec::new();

    {
        let mut writer = WalWriter::new(wal_buf, log_buf);
        for rec in records {
            writer.append(rec).unwrap();
        }
        let (w, l) = writer.into_inner();
        wal_buf = w;
        log_buf = l;
    }

    let mut reader = WalReader::from_vec(wal_buf);
    
    // Load logs from buffer
    let mut log_reader = pyroduct::format::wal::LogFrameReader::new(&log_buf[..]);
    reader.logs = log_reader.read_all_indexed();
    
    reader
}

// ---------------------------------------------------------------------------
// Lifecycle: create / write / open / recover
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_lifecycle_single_success() {
    let records = vec![make_success_record(0)];
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 1);

    match &recovered[0] {
        WalRecord::Success { row_index, .. } => assert_eq!(*row_index, 0),
        _ => panic!("expected Success"),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_wal_lifecycle_multiple_sequential() {
    let records = make_mixed_records(100);
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 100);

    for (i, rec) in recovered.iter().enumerate() {
        assert_eq!(rec.row_index(), i);
    }
}

// ---------------------------------------------------------------------------
// Roundtrip integrity — data must survive memory I/O
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_roundtrip_success_data() {
    let records = vec![make_success_record(42)];
    let reader = write_and_open_in_memory(&records);
    let frames: Vec<_> = reader.frames().collect();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].row_index, 42);

    // Verify the packet can be parsed back as a PyroRow
    let view = reader.view_at(frames[0].packet_offset).expect("view_at ok");
    let ref_ = view.py_ref();
    let row = PyroRow::expose_view(ref_).expect("parse ok");
    let row: PyroRow = (&*row).into();
    assert_eq!(row.get("id"), Some(&PyroValue::from(42i32)));
}

#[tracing_test::traced_test]
#[test]
fn test_wal_roundtrip_failure_data() {
    let records = vec![make_failure_record(7)];
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 1);

    match &recovered[0] {
        WalRecord::Failure { row_index, failure } => {
            assert_eq!(*row_index, 7);
            assert!(failure.result.is_err());
            assert!(
                failure
                    .result
                    .as_ref()
                    .unwrap_err()
                    .contains("row 7 failed")
            );
        }
        _ => panic!("expected Failure"),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_wal_roundtrip_mixed_types() {
    let records = vec![
        make_success_record(1),
        make_failure_record(2),
        make_success_record(3),
        make_failure_record(4),
        make_success_record(5),
    ];

    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 5);

    for (i, rec) in recovered.iter().enumerate() {
        let expected_idx = match &records[i] {
            WalRecord::Success { row_index, .. } => *row_index,
            WalRecord::Failure { row_index, .. } => *row_index,
        };
        match (&records[i], rec) {
            (WalRecord::Success { .. }, WalRecord::Success { row_index, .. }) => {
                assert_eq!(*row_index, expected_idx)
            }
            (WalRecord::Failure { .. }, WalRecord::Failure { row_index, .. }) => {
                assert_eq!(*row_index, expected_idx)
            }
            _ => panic!("row {} type mismatch", i),
        }
    }
}

// ---------------------------------------------------------------------------
// Frame iteration edge cases
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_empty_buffer() {
    let reader = WalReader::from_vec(vec![]);
    let frames: Vec<_> = reader.frames().collect();
    assert!(frames.is_empty());

    let recovered = reader.recover_all();
    assert!(recovered.is_empty());
}

#[tracing_test::traced_test]
#[test]
fn test_wal_large_row_indices() {
    let big_indices: Vec<usize> = vec![0, 100, 1000, 100_000, 1_000_000];
    let records: Vec<_> = big_indices.iter().map(|&idx| make_success_record(idx)).collect();
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 5);
    for (i, rec) in recovered.iter().enumerate() {
        assert_eq!(rec.row_index(), big_indices[i]);
    }
}

#[tracing_test::traced_test]
#[test]
fn test_wal_large_payload() {
    // Build a row with a big string payload
    let big = "X".repeat(10_000);
    let record = WalRecord::Success {
        row_index: 1,
        success: PyroSuccess {
            row: PyroRow::from([("payload", PyroValue::from(big.clone()))]).into_owned(),
            logs: PyroLogs::empty(),
        },
    };
    let reader = write_and_open_in_memory(&[record]);
    let frames: Vec<_> = reader.frames().collect();
    assert_eq!(frames.len(), 1);

    let view = reader.view_at(frames[0].packet_offset).expect("view_at");
    let ref_ = view.py_ref();
    let row = PyroRow::expose_view(ref_).expect("parse ok");
    let row: PyroRow = (&*row).into();
    let restored: String = row.get_value("payload").expect("payload is a string");
    assert_eq!(restored, big);
}

// ---------------------------------------------------------------------------
// View safety — ref counting and lifetime binding
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_multiple_views_from_same_reader() {
    let records: Vec<_> = (0..10).map(make_success_record).collect();
    let reader = write_and_open_in_memory(&records);
    let frames: Vec<_> = reader.frames().collect();

    // Get views for every frame
    let views: Vec<PyroView> = frames
        .iter()
        .map(|f| reader.view_at(f.packet_offset).expect("view_at"))
        .collect();

    // All views should be live and readable
    for (i, view) in views.iter().enumerate() {
        let ref_ = view.py_ref();
        let row = PyroRow::expose_view(ref_).expect("parse ok");
        let row: PyroRow = (&*row).into();
        
        assert_eq!(row.get("id"), Some(&PyroValue::from(i as i32)));
    }

    // Reader must still be alive (views depend on it)
    drop(views);
}

#[tracing_test::traced_test]
#[test]
fn test_wal_view_data_independence() {
    let records = vec![make_success_record(1), make_success_record(2)];
    let reader = write_and_open_in_memory(&records);
    let frames: Vec<_> = reader.frames().collect();
    let v1 = reader.view_at(frames[0].packet_offset).expect("v1");
    let v2 = reader.view_at(frames[1].packet_offset).expect("v2");

    let r1_ref = v1.py_ref();
    let r1 = PyroRow::expose_view(r1_ref).expect("r1");
    let r1: PyroRow = (&*r1).into(); 
    let r2_ref = v2.py_ref();
    let r2 = PyroRow::expose_view(r2_ref).expect("r2");
    let r2: PyroRow = (&*r2).into(); 
    assert_eq!(r1.get("id"), Some(&PyroValue::from(1i32)));
    assert_eq!(r2.get("id"), Some(&PyroValue::from(2i32)));

    // Views should be independent
    drop(v1);
    let r2_still_ref = v2.py_ref();
    let r2_still = PyroRow::expose_view(r2_still_ref).expect("r2 still valid");
    let r2_still: PyroRow = (&*r2_still).into(); 
    assert_eq!(r2_still.get("id"), Some(&PyroValue::from(2i32)));
}

// ---------------------------------------------------------------------------
// Drop safety — reader must not drop while views exist
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_reader_drops_after_views_dropped() {
    let records = vec![make_success_record(99)];
    let reader = write_and_open_in_memory(&records);
    let frames: Vec<_> = reader.frames().collect();
    {
        let _view = reader.view_at(frames[0].packet_offset).expect("view");
        // Reader and view coexist — should be fine
    }
    drop(reader);
}

// ---------------------------------------------------------------------------
// Concurrent access safety
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_reader_shared_across_threads() {
    let records = make_mixed_records(50);
    let reader = Arc::new(write_and_open_in_memory(&records));
    let barrier = Arc::new(Barrier::new(4));

    let mut handles = vec![];
    for _ in 0..4 {
        let r = Arc::clone(&reader);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let recovered = r.recover_all();
            assert_eq!(recovered.len(), 50);
            for (i, rec) in recovered.iter().enumerate() {
                assert_eq!(rec.row_index(), i);
            }
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }
}

#[tracing_test::traced_test]
#[test]
fn test_wal_reader_iterate_concurrently() {
    let records = make_mixed_records(100);
    let reader = Arc::new(write_and_open_in_memory(&records));
    let barrier = Arc::new(Barrier::new(3));

    let mut handles = vec![];
    for _ in 0..3 {
        let r = Arc::clone(&reader);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let frames: Vec<_> = r.frames().collect();
            assert_eq!(frames.len(), 100);
            for (i, f) in frames.iter().enumerate() {
                assert_eq!(f.row_index, i);
            }
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }
}

// ---------------------------------------------------------------------------
// Memory persistence
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_data_persists_across_open() {
    let records: Vec<_> = (0..20).map(make_success_record).collect();
    
    // Simulate first session
    let mut wal_buf = Vec::new();
    let mut log_buf = Vec::new();
    {
        let mut writer = WalWriter::new(wal_buf, log_buf);
        for rec in &records {
            writer.append(rec).unwrap();
        }
        let (w, l) = writer.into_inner();
        wal_buf = w;
        log_buf = l;
    }

    // Second "open" — all data should survive
    let mut reader = WalReader::from_vec(wal_buf);
    let mut log_reader = pyroduct::format::wal::LogFrameReader::new(&log_buf[..]);
    reader.logs = log_reader.read_all_indexed();
    
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 20);

    for (i, rec) in recovered.iter().enumerate() {
        assert_eq!(rec.row_index(), i);
    }
}

#[tracing_test::traced_test]
#[test]
fn test_wal_incremental_writes() {
    let mut wal_buf = Vec::new();
    let mut log_buf = Vec::new();

    // Write in two separate writer sessions
    {
        let mut writer = WalWriter::new(std::mem::take(&mut wal_buf), std::mem::take(&mut log_buf));
        writer.append(&make_success_record(0)).unwrap();
        writer.append(&make_success_record(1)).unwrap();
        let (w, l) = writer.into_inner();
        wal_buf = w;
        log_buf = l;
    }
    {
        let mut writer = WalWriter::new(std::mem::take(&mut wal_buf), std::mem::take(&mut log_buf));
        writer.append(&make_success_record(2)).unwrap();
        writer.append(&make_success_record(3)).unwrap();
        let (w, l) = writer.into_inner();
        wal_buf = w;
        log_buf = l;
    }

    let mut reader = WalReader::from_vec(wal_buf);
    let mut log_reader = pyroduct::format::wal::LogFrameReader::new(&log_buf[..]);
    reader.logs = log_reader.read_all_indexed();
    
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 4);

    for (i, rec) in recovered.iter().enumerate() {
        assert_eq!(rec.row_index(), i);
    }
}

// ---------------------------------------------------------------------------
// Logs integration
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_logs_survive_recovery() {
    let records = vec![make_success_record(42)];
    let reader = write_and_open_in_memory(&records);

    // Logs should be attached to recovered records
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 1);

    if let WalRecord::Success { row_index, success } = &recovered[0] {
        assert_eq!(*row_index, 42);
        assert!(!success.logs.module_logs.is_empty());
        assert_eq!(success.logs.module_logs[0], "processing row 42");
    } else {
        panic!("expected Success");
    }
}

// ---------------------------------------------------------------------------
// Corruption / edge cases
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_from_vec_empty() {
    let reader = WalReader::from_vec(vec![]);
    let frames: Vec<_> = reader.frames().collect();
    assert!(frames.is_empty());
}

#[tracing_test::traced_test]
#[test]
fn test_wal_from_vec_partial_prefix() {
    // Just 4 bytes — not enough for a full 16-byte prefix
    let reader = WalReader::from_vec(vec![0, 0, 0, 0]);
    let frames: Vec<_> = reader.frames().collect();
    assert!(frames.is_empty());
}

#[tracing_test::traced_test]
#[test]
fn test_wal_from_vec_truncated_packet() {
    // Write a 16-byte prefix, then only a few bytes of payload
    let mut data = Vec::with_capacity(16 + 8);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(b"short");
    // Not enough for a valid PyroRef
    let reader = WalReader::from_vec(data);
    let frames: Vec<_> = reader.frames().collect();
    assert!(frames.is_empty());
}

// ---------------------------------------------------------------------------
// Path handling
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_paths_not_used_in_memory() {
    let records = vec![make_success_record(0)];
    let reader = write_and_open_in_memory(&records);
    assert!(reader.path.is_none());
}

// ---------------------------------------------------------------------------
// Large-scale stress
// ---------------------------------------------------------------------------

#[tracing_test::traced_test]
#[test]
fn test_wal_large_batch() {
    let count = 10_000;
    let records: Vec<_> = (0..count).map(make_success_record).collect();
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), count);

    // Spot-check first, middle, last
    assert_eq!(recovered[0].row_index(), 0);
    assert_eq!(recovered[count / 2].row_index(), count / 2);
    assert_eq!(recovered[count - 1].row_index(), count - 1);
}
