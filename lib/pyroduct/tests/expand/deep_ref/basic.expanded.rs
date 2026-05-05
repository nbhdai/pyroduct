//! Test DeepRef with basic types
use pyroduct::format::DeepRef;
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
impl ::pyroduct::format::DeepRef for User {
    type Ref<'a> = UserRef<'a>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        UserRef {
            id: self.id,
            username: self.username.as_str(),
            score: self.score,
        }
    }
}
impl<'a> From<UserRef<'a>> for User {
    fn from(reference: UserRef<'a>) -> Self {
        Self {
            id: reference.id,
            username: reference.username.to_string(),
            score: reference.score,
        }
    }
}
impl<'a> From<&'a UserRef<'a>> for User {
    fn from(reference: &'a UserRef<'a>) -> Self {
        Self {
            id: reference.id,
            username: reference.username.to_string(),
            score: reference.score,
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
