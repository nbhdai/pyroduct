use arrow_scalars::{ArchivedArrowRow, ArrowRow, ArrowValue, DeepRef, FromRow, ToRow};
use rkyv::{Archive, Deserialize, Serialize};

// ============================================================================
// Example 1: Basic struct with primitive types
// ============================================================================

#[derive(ToRow, FromRow, DeepRef, Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub age: i32,
    pub is_active: bool,
}

#[test]
fn test_basic_to_arrow() {
    let user = User {
        id: 42,
        username: "alice".to_string(),
        age: 30,
        is_active: true,
    };

    // Convert to borrowed ArrowRow (field names are static strings)
    let row = user.to_row();
    assert_eq!(row.get("id"), Some(&ArrowValue::U32(42)));
    assert_eq!(row.get("username"), Some(&ArrowValue::from("alice")));
}

// ============================================================================
// Example 2: Struct with Vec and Option
// ============================================================================

#[derive(ToRow, FromRow, DeepRef, Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SensorReading {
    pub sensor_id: u32,
    pub timestamp: i64,
    pub readings: Vec<f64>,
    pub location: String,
    pub error_code: Option<i32>,
}

#[test]
fn test_vec_and_option() {
    let reading = SensorReading {
        sensor_id: 100,
        timestamp: 1234567890,
        readings: vec![23.5, 23.6, 23.7],
        location: "Building A".to_string(),
        error_code: None,
    };

    // Convert to ArrowRow
    let row = reading.to_row();

    // Verify primitive list
    if let Some(ArrowValue::PrimitiveList(list)) = row.get("readings") {
        // Readings are borrowed from the original vec
        println!("Readings: {:?}", list);
    }

    // Verify Option<i32> becomes Null
    assert_eq!(row.get("error_code"), Some(&ArrowValue::Null));

    // With error code
    let reading_with_error = SensorReading {
        error_code: Some(404),
        ..reading
    };
    let row = reading_with_error.to_row();
    assert_eq!(row.get("error_code"), Some(&ArrowValue::I32(404)));
}

// ============================================================================
// Example 3: Nested structs
// ============================================================================

#[derive(ToRow, FromRow, DeepRef, Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub zip_code: u32,
}

#[derive(ToRow, FromRow, DeepRef, Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Person {
    pub name: String,
    pub age: i32,
    pub address: Address,
}

#[test]
fn test_nested_structs() {
    let person = Person {
        name: "Bob".to_string(),
        age: 25,
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip_code: 12345,
        },
    };

    // Convert to ArrowRow
    let row = person.to_row();

    // Nested struct becomes ArrowValue::Group
    if let Some(ArrowValue::Group(addr_row)) = row.get("address") {
        assert_eq!(addr_row.get("city"), Some(&ArrowValue::from("Springfield")));
        assert_eq!(addr_row.get("zip_code"), Some(&ArrowValue::U32(12345)));
    }

    // Round-trip through ArrowValue
    let arrow_value = person.to_row();
    let person_ref = PersonRef::from_row(&arrow_value).unwrap();
    assert_eq!(person_ref.name, "Bob");
    assert_eq!(person_ref.address.city, "Springfield");
}

// ============================================================================
// Example 4: Vec of nested structs
// ============================================================================

#[derive(ToRow, FromRow, DeepRef, Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Order {
    pub order_id: u64,
    pub items: Vec<OrderItem>,
    pub total: f64,
}

#[derive(ToRow, FromRow, DeepRef, Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderItem {
    pub product_id: u32,
    pub quantity: i32,
    pub price: f64,
}

#[test]
fn test_vec_of_structs() {
    let order = Order {
        order_id: 999,
        items: vec![
            OrderItem {
                product_id: 1,
                quantity: 2,
                price: 10.50,
            },
            OrderItem {
                product_id: 2,
                quantity: 1,
                price: 25.00,
            },
        ],
        total: 46.00,
    };

    // Convert to ArrowRow
    let row = order.to_row();

    // Vec of structs becomes ArrowValue::List of ArrowValue::Group
    if let Some(ArrowValue::List(items)) = row.get("items") {
        assert_eq!(items.len(), 2);

        if let ArrowValue::Group(item_row) = &items[0] {
            assert_eq!(item_row.get("product_id"), Some(&ArrowValue::U32(1)));
            assert_eq!(item_row.get("quantity"), Some(&ArrowValue::I32(2)));
        }
    }
}

// ============================================================================
// Example 5: Comparison with manual ArrowRow construction
// ============================================================================

#[test]
fn test_vs_manual_construction() {
    let user = User {
        id: 42,
        username: "alice".to_string(),
        age: 30,
        is_active: true,
    };

    // Using ToRow (automatic, uses static strings for field names)
    let row_auto = user.to_row();

    // Manual construction (what you had to do before)
    let row_manual = ArrowRow::from([
        ("id", ArrowValue::U32(42)),
        ("username", ArrowValue::from("alice")),
        ("age", ArrowValue::I32(30)),
        ("is_active", ArrowValue::Bool(true)),
    ]);

    // Both produce equivalent results
    assert_eq!(row_auto, row_manual);
}

// ============================================================================
// Example 6: Serialization workflow
// ============================================================================
#[test]
fn test_serialization_workflow() {
    let sensor = SensorReading {
        sensor_id: 42,
        timestamp: 1704067200000,
        readings: vec![23.1, 23.3, 23.5],
        location: "Lab 1".to_string(),
        error_code: None,
    };

    // 1. Convert owned struct to ArrowRow using ToRow
    // Fix: Store the result in a binding to extend its lifetime
    let arrow_row = sensor.to_row();

    // 2. Serialize with rkyv
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&arrow_row).expect("Failed to serialize");

    // 3. Access archived data (zero-copy)
    let archived =
        rkyv::access::<ArchivedArrowRow, rkyv::rancor::Error>(&bytes).expect("Failed to access");

    // 4. Convert to ArrowValue
    let row = ArrowRow::from(archived);

    // 5. Extract reference using FromRow (from FromRow derive)
    let sensor_ref = SensorReadingRef::from_row(&row).expect("Failed to create ref");

    // 6. Verify all fields match
    assert_eq!(sensor_ref.sensor_id, sensor.sensor_id);
    assert_eq!(sensor_ref.location, sensor.location.as_str());
    assert_eq!(sensor_ref.readings, sensor.readings.as_slice());
}
