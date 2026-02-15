//! Test DeepRef with basic types
use pyroduct::DeepRef;
struct User {
    id: u32,
    username: String,
    score: i32,
}
pub struct UserRef<'deep_ref_lifetime> {
    id: <u32 as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    username: <String as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    score: <i32 as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
}
impl ::pyroduct::DeepRef for User {
    type Ref<'deep_ref_lifetime> = UserRef<'deep_ref_lifetime>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        UserRef {
            id: <u32 as ::pyroduct::DeepRef>::as_deep_ref(&self.id),
            username: <String as ::pyroduct::DeepRef>::as_deep_ref(&self.username),
            score: <i32 as ::pyroduct::DeepRef>::as_deep_ref(&self.score),
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
