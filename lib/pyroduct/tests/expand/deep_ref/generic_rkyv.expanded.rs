//! Test FromRow with nested structs
use pyroduct::{DeepRef, DeepRefArchived};
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
impl ::pyroduct::DeepRef for ::pyroduct::rkyv_8::rkyv::Archived<Address> {
    type Ref<'deep_ref_lifetime> = AddressRef<'deep_ref_lifetime>;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        AddressRef {
            street: <<String as ::pyroduct::rkyv_8::rkyv::Archive>::Archived as ::pyroduct::DeepRef>::as_deep_ref(
                &self.street,
            ),
            city: <<String as ::pyroduct::rkyv_8::rkyv::Archive>::Archived as ::pyroduct::DeepRef>::as_deep_ref(
                &self.city,
            ),
            zip: <<u32 as ::pyroduct::rkyv_8::rkyv::Archive>::Archived as ::pyroduct::DeepRef>::as_deep_ref(
                &self.zip,
            ),
        }
    }
}
struct Person<T> {
    name: String,
    age: i32,
    address: T,
}
pub struct PersonRef<'deep_ref_lifetime, TRef> {
    name: <String as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    age: <i32 as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    address: TRef,
}
impl<T: ::pyroduct::DeepRef> ::pyroduct::DeepRef for Person<T> {
    type Ref<'deep_ref_lifetime> = PersonRef<
        'deep_ref_lifetime,
        <T as ::pyroduct::DeepRef>::Ref<'deep_ref_lifetime>,
    >
    where
        T: 'deep_ref_lifetime;
    fn as_deep_ref(&self) -> Self::Ref<'_> {
        PersonRef {
            name: <String as ::pyroduct::DeepRef>::as_deep_ref(&self.name),
            age: <i32 as ::pyroduct::DeepRef>::as_deep_ref(&self.age),
            address: <T as ::pyroduct::DeepRef>::as_deep_ref(&self.address),
        }
    }
}
fn main() {
    let person = Person::<Address> {
        name: "Bob".to_string(),
        age: 30,
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: 12345,
        },
    };
    let p_ref = person.as_deep_ref();
    match (&p_ref.name, &"Bob") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p_ref.age, &30) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p_ref.address.street, &"123 Main St") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p_ref.address.city, &"Springfield") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&p_ref.address.zip, &12345) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
