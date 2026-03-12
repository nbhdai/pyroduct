//! Test DeepRef with basic types

use pyroduct::format::{
    DeepRef, DeepRefArchived,
    rkyv_8::rkyv::{self, Archive, Deserialize, Serialize},
};

#[derive(DeepRef, DeepRefArchived, Archive, Serialize, Deserialize)]
struct User {
    id: u32,
    username: String,
    score: i32,
}

fn main() {
    let user = User {
        id: 42,
        username: "alice".to_string(),
        score: 100,
    };

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&user).unwrap();
    let user_rkyv = rkyv::access::<ArchivedUser, rkyv::rancor::Error>(&bytes.as_slice()).unwrap();

    // as_deep_ref should convert to borrowed view
    let user_ref = user_rkyv.as_deep_ref();

    assert_eq!(user_ref.id, 42);
    assert_eq!(user_ref.username, "alice");
    assert_eq!(user_ref.score, 100);

    // Verify string is actually borrowed
    let _: &str = user_ref.username;

    // Original still exists and is unchanged
    assert_eq!(user.username, "alice");
}
