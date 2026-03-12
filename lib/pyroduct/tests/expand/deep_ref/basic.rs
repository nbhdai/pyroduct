//! Test DeepRef with basic types

use pyroduct::format::DeepRef;

#[derive(DeepRef)]
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

    // as_deep_ref should convert to borrowed view
    let user_ref = user.as_deep_ref();
}
