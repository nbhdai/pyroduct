//! Expansion test for DeepRef derive
use arrow_scalars::DeepRef;
struct TestStruct {
    id: u32,
    name: String,
}
pub struct TestStructRef<'a> {
    id: u32,
    name: &'a str,
}
#[automatically_derived]
impl<'a> ::core::fmt::Debug for TestStructRef<'a> {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "TestStructRef",
            "id",
            &self.id,
            "name",
            &&self.name,
        )
    }
}
#[automatically_derived]
impl<'a> ::core::clone::Clone for TestStructRef<'a> {
    #[inline]
    fn clone(&self) -> TestStructRef<'a> {
        TestStructRef {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
        }
    }
}
#[automatically_derived]
impl<'a> ::core::marker::StructuralPartialEq for TestStructRef<'a> {}
#[automatically_derived]
impl<'a> ::core::cmp::PartialEq for TestStructRef<'a> {
    #[inline]
    fn eq(&self, other: &TestStructRef<'a>) -> bool {
        self.id == other.id && self.name == other.name
    }
}
impl ::arrow_scalars::DeepRef for TestStruct {
    type Ref<'a> = TestStructRef<'a>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        TestStructRef {
            id: self.id,
            name: self.name.as_str(),
        }
    }
}
