//! Test DeepRef with basic types
use pyroduct::DeepRef;
struct User {
    id: u32,
    username: String,
    score: i32,
}
pub struct UserRef<'a> {
    id: u32,
    username: &'a str,
    score: i32,
}
impl ::pyroduct::DeepRef for User {
    type Ref<'a> = UserRef<'a>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        UserRef {
            id: self.id,
            username: self.username.as_str(),
            score: self.score,
        }
    }
}
fn main() {
    let user = User {
        id: 42,
        username: "alice".to_string(),
        score: 100,
    };
    let user_ref = user.as_deep_ref();
}
