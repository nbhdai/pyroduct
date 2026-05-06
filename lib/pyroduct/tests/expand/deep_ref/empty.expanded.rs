//! Test FromRow with empty struct (edge case)
use pyroduct::format::DeepRef;
struct Empty {}
pub struct EmptyRef<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}
impl ::pyroduct::format::DeepRef for Empty {
    type Ref<'a> = EmptyRef<'a>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        EmptyRef {
            _phantom: std::marker::PhantomData,
        }
    }
}
impl<'a> ::pyroduct::format::FromRef<EmptyRef<'a>> for Empty {
    fn from_ref(reference: &EmptyRef<'a>) -> Self {
        Self {}
    }
}
fn main() {
    let empty = Empty {};
    let _e = empty.as_deep_ref();
}
