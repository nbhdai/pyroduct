use pyroduct::format::header::PyroData;
use pyroduct::format::wal::{WalReader, WalWriter};
use pyroduct::format::{Bridgeable, PyroRow, PyroValue};
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::tempdir;

fn make_row(row_index: usize) -> PyroRow<'static> {
    PyroRow::from([
        ("id", PyroValue::from(row_index as i32)),
        ("name", PyroValue::from("test")),
    ])
    .into_owned()
}

fn write_and_open_in_memory(records: &[(usize, PyroRow<'static>)]) -> WalReader {
    let mut wal_buf = Vec::new();

    {
        let mut writer = WalWriter::new(wal_buf);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for (idx, row) in records {
            let vec = row.clone().into_owned().ship().unwrap();
            rt.block_on(writer.append(*idx, vec.py_ref())).unwrap();
        }
        wal_buf = writer.into_inner();
    }

    WalReader::from_vec(wal_buf)
}

#[test]
fn test_wal_lifecycle_single_success() {
    let row = make_row(0);
    let records = vec![(42, row)];
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 1);

    let (idx, view) = &recovered[0];
    assert_eq!(*idx, 42);

    let py_ref = view.py_ref();
    let parsed_ref = PyroRow::expose_view(py_ref).unwrap();
    let parsed_row: PyroRow = (&*parsed_ref).into();
    assert_eq!(parsed_row.get_value::<i32>("id").unwrap(), 0);
    assert_eq!(parsed_row.get_value::<String>("name").unwrap(), "test");
}

#[test]
fn test_wal_lifecycle_multiple_sequential() {
    let count = 100;
    let records: Vec<_> = (0..count).map(|i| (i, make_row(i))).collect();
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), count);

    for (i, (idx, view)) in recovered.iter().enumerate() {
        assert_eq!(*idx, i);
        let py_ref = view.py_ref();
        let parsed_ref = PyroRow::expose_view(py_ref).unwrap();
        let parsed_row: PyroRow = (&*parsed_ref).into();
        assert_eq!(parsed_row.get_value::<i32>("id").unwrap(), i as i32);
    }
}

#[test]
fn test_wal_empty_buffer() {
    let reader = WalReader::from_vec(vec![]);
    let recovered = reader.recover_all();
    assert!(recovered.is_empty());
}

#[test]
fn test_wal_large_row_indices() {
    let big_indices = vec![0, 100, 1000, 100_000, 1_000_000];
    let records: Vec<_> = big_indices
        .iter()
        .map(|&idx| (idx, make_row(idx)))
        .collect();
    let reader = write_and_open_in_memory(&records);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 5);
    for (i, (idx, _)) in recovered.iter().enumerate() {
        assert_eq!(*idx, big_indices[i]);
    }
}

#[test]
fn test_wal_large_payload() {
    let big = "X".repeat(10_000);
    let row = PyroRow::from([("payload", PyroValue::from(big.clone()))]).into_owned();
    let reader = write_and_open_in_memory(&[(42, row)]);
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), 1);

    let (idx, view) = &recovered[0];
    assert_eq!(*idx, 42);
    let py_ref = view.py_ref();
    let parsed_ref = PyroRow::expose_view(py_ref).unwrap();
    let parsed_row: PyroRow = (&*parsed_ref).into();
    assert_eq!(parsed_row.get_value::<String>("payload").unwrap(), big);
}

#[test]
fn test_wal_reader_shared_across_threads() {
    let count = 50;
    let records: Vec<_> = (0..count).map(|i| (i, make_row(i))).collect();
    let reader = Arc::new(write_and_open_in_memory(&records));
    let barrier = Arc::new(Barrier::new(4));

    let mut handles = vec![];
    for _ in 0..4 {
        let r: Arc<WalReader> = Arc::clone(&reader);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let recovered = r.recover_all();
            assert_eq!(recovered.len(), count);
            for (i, (idx, _)) in recovered.iter().enumerate() {
                assert_eq!(*idx, i);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_wal_data_persists_across_open() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");

    let count = 20;
    let records: Vec<_> = (0..count).map(|i| (i, make_row(i))).collect();

    // Write session
    {
        let mut writer = WalWriter::open(&wal_path).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for (idx, row) in &records {
            let vec = row.clone().into_owned().ship().unwrap();
            rt.block_on(writer.append(*idx, vec.py_ref())).unwrap();
        }
    }

    // Read session
    let reader = WalReader::open(&wal_path).unwrap();
    let recovered = reader.recover_all();
    assert_eq!(recovered.len(), count);

    for (i, (idx, view)) in recovered.iter().enumerate() {
        assert_eq!(*idx, i);
        let py_ref = view.py_ref();
        let parsed_ref = PyroRow::expose_view(py_ref).unwrap();
        let parsed_row: PyroRow = (&*parsed_ref).into();
        assert_eq!(parsed_row.get_value::<i32>("id").unwrap(), i as i32);
    }
}
