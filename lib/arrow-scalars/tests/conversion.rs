use arrow_scalars::{
    ArchivedArrowRow, ArrowRow, ArrowValue, DeepRef, FromRow, FromValue, PrimitiveValueList, ToRow,
};
use rkyv::{Archive, Deserialize, Serialize};

// Define a test struct with various field types
#[derive(FromRow, DeepRef, ToRow, Archive, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SensorReading {
    pub sensor_id: u32,
    pub timestamp: i64,
    pub temperature: f32,
    pub humidity: f32,
    pub location: String,
    pub readings: Vec<f64>,
    pub is_active: bool,
    pub error_code: Option<i32>,
}

#[test]
fn test_archive_to_deep_ref_roundtrip() {
    // 1. Create the original owned struct
    let original = SensorReading {
        sensor_id: 42,
        timestamp: 1704067200000,
        temperature: 23.5,
        humidity: 65.0,
        location: "Building A - Floor 2".to_string(),
        readings: vec![23.1, 23.3, 23.5, 23.7, 23.9],
        is_active: true,
        error_code: None,
    };

    // 2. Create an equivalent ArrowRow manually
    let arrow_row = ArrowRow::from([
        ("sensor_id", ArrowValue::U32(42)),
        ("timestamp", ArrowValue::I64(1704067200000)),
        ("temperature", ArrowValue::F32(23.5)),
        ("humidity", ArrowValue::F32(65.0)),
        ("location", ArrowValue::from("Building A - Floor 2")),
        (
            "readings",
            ArrowValue::from(&[23.1f64, 23.3, 23.5, 23.7, 23.9][..]),
        ),
        ("is_active", ArrowValue::Bool(true)),
        ("error_code", ArrowValue::Null),
    ]);

    // 3. Serialize the ArrowRow using rkyv
    let bytes =
        rkyv::to_bytes::<rkyv::rancor::Error>(&arrow_row).expect("Failed to serialize ArrowRow");

    // 4. Access the archived data (zero-copy)
    let archived_row = rkyv::access::<ArchivedArrowRow, rkyv::rancor::Error>(&bytes)
        .expect("Failed to access archived ArrowRow");

    // 5. Convert archived row to ArrowValue with proper lifetime
    let arrow_value = ArrowValue::Group(ArrowRow::from(archived_row));

    // 6. Extract the deep reference using the derived FromRow trait
    let sensor_ref = SensorReadingRef::from_value(&arrow_value)
        .expect("Failed to create SensorReadingRef from archived ArrowValue");

    // 7. Verify all fields match the original
    assert_eq!(sensor_ref.sensor_id, original.sensor_id);
    assert_eq!(sensor_ref.timestamp, original.timestamp);
    assert_eq!(sensor_ref.temperature, original.temperature);
    assert_eq!(sensor_ref.humidity, original.humidity);
    assert_eq!(sensor_ref.location, original.location.as_str());
    assert_eq!(sensor_ref.readings, original.readings.as_slice());
    assert_eq!(sensor_ref.is_active, original.is_active);
    assert_eq!(sensor_ref.error_code, original.error_code);
}

#[test]
fn test_nested_struct_archive_conversion() {
    // Test with nested structures
    #[derive(FromRow, DeepRef, Archive, Serialize, Deserialize, PartialEq, Debug, Clone)]
    pub struct DeviceInfo {
        pub device_id: String,
        pub firmware_version: String,
    }

    #[derive(FromRow, DeepRef, Archive, Serialize, Deserialize, PartialEq, Debug, Clone)]
    pub struct ComplexReading {
        pub id: u64,
        pub device: DeviceInfo,
        pub values: Vec<i32>,
    }

    let original = ComplexReading {
        id: 999,
        device: DeviceInfo {
            device_id: "DEV-001".to_string(),
            firmware_version: "v2.1.3".to_string(),
        },
        values: vec![10, 20, 30, 40],
    };

    // Create equivalent ArrowRow with nested structure
    let arrow_row = ArrowRow::from([
        ("id", ArrowValue::U64(999)),
        (
            "device",
            ArrowValue::Group(ArrowRow::from([
                ("device_id", ArrowValue::from("DEV-001")),
                ("firmware_version", ArrowValue::from("v2.1.3")),
            ])),
        ),
        ("values", ArrowValue::from(&[10i32, 20, 30, 40][..])),
    ]);

    // Serialize and access
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&arrow_row).expect("Failed to serialize");

    let archived =
        rkyv::access::<ArchivedArrowRow, rkyv::rancor::Error>(&bytes).expect("Failed to access");

    let row = ArrowRow::from(archived);

    // Extract deep reference
    let reading_ref =
        ComplexReadingRef::from_row(&row).expect("Failed to create ComplexReadingRef");

    // Verify nested fields
    assert_eq!(reading_ref.id, original.id);
    assert_eq!(
        reading_ref.device.device_id,
        original.device.device_id.as_str()
    );
    assert_eq!(
        reading_ref.device.firmware_version,
        original.device.firmware_version.as_str()
    );
    assert_eq!(reading_ref.values, original.values.as_slice());
}

#[test]
fn test_primitive_list_types() {
    // Test all primitive list types to ensure transmutation works correctly
    let arrow_row = ArrowRow::from([
        ("u8_list", ArrowValue::from(&[1u8, 2, 3][..])),
        ("u16_list", ArrowValue::from(&[100u16, 200, 300][..])),
        ("u32_list", ArrowValue::from(&[1000u32, 2000, 3000][..])),
        ("u64_list", ArrowValue::from(&[10000u64, 20000, 30000][..])),
        ("i8_list", ArrowValue::from(&[-1i8, -2, -3][..])),
        ("i16_list", ArrowValue::from(&[-100i16, -200, -300][..])),
        ("i32_list", ArrowValue::from(&[-1000i32, -2000, -3000][..])),
        (
            "i64_list",
            ArrowValue::from(&[-10000i64, -20000, -30000][..]),
        ),
        ("f32_list", ArrowValue::from(&[1.1f32, 2.2, 3.3][..])),
        ("f64_list", ArrowValue::from(&[10.1f64, 20.2, 30.3][..])),
    ]);

    // Serialize
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&arrow_row).expect("Failed to serialize");

    let archived =
        rkyv::access::<ArchivedArrowRow, rkyv::rancor::Error>(&bytes).expect("Failed to access");

    let converted = ArrowRow::from(archived);

    // Verify each list type
    if let Some(ArrowValue::PrimitiveList(PrimitiveValueList::U8(list))) = converted.get("u8_list")
    {
        assert_eq!(list.as_ref(), &[1u8, 2, 3]);
    } else {
        panic!("u8_list not found or wrong type");
    }

    if let Some(ArrowValue::PrimitiveList(PrimitiveValueList::U16(list))) =
        converted.get("u16_list")
    {
        assert_eq!(list.as_ref(), &[100u16, 200, 300]);
    } else {
        panic!("u16_list not found or wrong type");
    }

    if let Some(ArrowValue::PrimitiveList(PrimitiveValueList::I32(list))) =
        converted.get("i32_list")
    {
        assert_eq!(list.as_ref(), &[-1000i32, -2000, -3000]);
    } else {
        panic!("i32_list not found or wrong type");
    }

    if let Some(ArrowValue::PrimitiveList(PrimitiveValueList::F64(list))) =
        converted.get("f64_list")
    {
        assert_eq!(list.as_ref(), &[10.1f64, 20.2, 30.3]);
    } else {
        panic!("f64_list not found or wrong type");
    }
}
