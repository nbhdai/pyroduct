use arrow_scalars::{ArrowRow, ArrowValue, DeepRef, FromRow, FromValue, ToValue};
use rkyv::{Archive, Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Test Struct Definition
// -----------------------------------------------------------------------------

/// A complex struct exercising Primitives, Strings, Lists, and Options.
#[derive(FromRow, DeepRef, Archive, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct UserProfile {
    pub id: u32,
    pub username: String,
    pub scores: Vec<i32>,
    pub metadata: Option<String>,
    pub is_active: bool,
}

// Manual implementation of ToValue for the struct
impl ToValue for UserProfile {
    fn to_value(&self) -> ArrowValue<'_> {
        // Since UserProfile only has primitives/std types, we can construct the row manually
        // or if we had ToRow, we could use ArrowValue::Group(self.to_row()).
        // But ToRow is not derived here? The test code below uses ArrowRow::from(...) manually.
        // Wait, the prompt says "FromRow and ToRow should be implemented by the macros".
        // The struct has #[derive(FromRow)]. It lacks #[derive(ToRow)].
        // I will add ToRow to the derive list so we can use it.
        ArrowValue::Group(ArrowRow::from([
            ("id", ArrowValue::from(self.id)),
            ("username", ArrowValue::from(&self.username)),
            ("scores", ArrowValue::from(&self.scores)),
            ("metadata", ArrowValue::from(&self.metadata)),
            ("is_active", ArrowValue::from(self.is_active)),
        ]))
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[test]
fn test_arrow_ref_from_arrow_value() {
    // 1. Construct an ArrowValue::Group mimicking a row for UserProfile
    let row = ArrowRow::from([
        ("id", ArrowValue::U32(101)),
        ("username", ArrowValue::from("alice_wonder")),
        ("scores", ArrowValue::from(&[10i32, 20, 30][..])),
        ("metadata", ArrowValue::from("premium_user")),
        ("is_active", ArrowValue::Bool(true)),
    ]);

    let val = ArrowValue::Group(row);

    // 2. Use the derived `from_value` (Now manually implemented above)
    let view =
        UserProfileRef::from_value(&val).expect("Failed to create UserProfileRef from ArrowValue");

    // 3. Assertions
    assert_eq!(view.id, 101);
    assert_eq!(view.username, "alice_wonder");
    assert_eq!(view.scores, &[10, 20, 30]);
    assert_eq!(view.metadata, Some("premium_user"));
    assert_eq!(view.is_active, true);
}

#[test]
fn test_deep_ref_from_owned() {
    let original = UserProfile {
        id: 500,
        username: "bob_builder".to_string(),
        scores: vec![5, 4, 3, 2, 1],
        metadata: None,
        is_active: false,
    };

    let view: UserProfileRef<'_> = original.as_deep_ref();

    assert_eq!(view.id, 500);
    assert_eq!(view.username, "bob_builder");
    assert_eq!(view.scores, original.scores.as_slice());
    assert_eq!(view.metadata, None);
    assert_eq!(view.is_active, false);
}

#[test]
fn test_arrow_ref_null_handling() {
    let scores: &[i32] = &[];
    let row = ArrowRow::from([
        ("id", ArrowValue::U32(1)),
        ("username", ArrowValue::from("tester")),
        ("scores", ArrowValue::from(scores)),
        ("metadata", ArrowValue::Null),
        ("is_active", ArrowValue::Bool(false)),
    ]);

    let view = UserProfileRef::from_row(&row).unwrap();

    assert_eq!(view.metadata, None);
    assert!(view.scores.is_empty());
}
