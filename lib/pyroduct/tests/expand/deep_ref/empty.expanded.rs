//! Test FromRow with empty struct (edge case)
use pyroduct::DeepRef;
struct Empty {}
pub struct EmptyRef<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}
impl ::pyroduct::DeepRef for Empty {
    type Ref<'a> = EmptyRef<'a>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        EmptyRef {
            _phantom: std::marker::PhantomData,
        }
    }
}
fn main() {
    let empty = Empty {};
    let _e = empty.as_deep_ref();
}
