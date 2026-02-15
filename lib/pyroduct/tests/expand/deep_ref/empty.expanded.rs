//! Test FromRow with empty struct (edge case)
use pyroduct::DeepRef;
struct Empty {}
pub struct EmptyRef<'deep_ref_lifetime> {
    _phantom: std::marker::PhantomData<&'deep_ref_lifetime ()>,
}
impl ::pyroduct::DeepRef for Empty {
    type Ref<'deep_ref_lifetime> = EmptyRef<'deep_ref_lifetime>;
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
