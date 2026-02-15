//! Test FromRow with nested structs
use pyroduct::DeepRef;
struct Address {
    street: String,
    city: String,
    zip: u32,
}
pub struct AddressRef<'deep_ref_lifetime> {
    street: <String as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    city: <String as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    zip: <u32 as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
}
impl ::pyroduct::DeepRef for Address {
    type Ref<'deep_ref_lifetime> = AddressRef<'deep_ref_lifetime>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        AddressRef {
            street: <String as ::pyroduct::DeepRef>::as_deep_ref(&self.street),
            city: <String as ::pyroduct::DeepRef>::as_deep_ref(&self.city),
            zip: <u32 as ::pyroduct::DeepRef>::as_deep_ref(&self.zip),
        }
    }
}
struct Person<S, T> {
    name: S,
    age: i32,
    address: T,
}
pub struct PersonRef<'deep_ref_lifetime, SRef, TRef> {
    name: SRef,
    age: <i32 as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    address: TRef,
}
impl<S: ::pyroduct::DeepRef, T: ::pyroduct::DeepRef> ::pyroduct::DeepRef
for Person<S, T> {
    type Ref<'deep_ref_lifetime> = PersonRef<
        'deep_ref_lifetime,
        <S as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
        <T as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    >
    where
        S: 'deep_ref_lifetime,
        T: 'deep_ref_lifetime;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        PersonRef {
            name: <S as ::pyroduct::DeepRef>::as_deep_ref(&self.name),
            age: <i32 as ::pyroduct::DeepRef>::as_deep_ref(&self.age),
            address: <T as ::pyroduct::DeepRef>::as_deep_ref(&self.address),
        }
    }
}
fn main() {
    let person = Person::<String, Address> {
        name: "Bob".to_string(),
        age: 30,
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: 12345,
        },
    };
    let p_ref = person.as_deep_ref();
}
