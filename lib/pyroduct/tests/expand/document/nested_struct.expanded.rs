use pyroduct::format::Document;
/// Inner struct
struct Inner {
    value: i64,
}
impl ::pyroduct::format::value::TypeableRow for Inner {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            {
                                let field = ::pyroduct::format::value::PyroField::<
                                    'static,
                                >::new(
                                    "value",
                                    <i64 as ::pyroduct::format::value::Typeable>::pyro_type(),
                                    <i64 as ::pyroduct::format::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                        ],
                    ),
                ),
            ),
            documentation: Some(::std::borrow::Cow::Borrowed("Inner struct")),
        }
    }
}
/// Outer struct
struct Outer {
    inner: Inner,
    count: u32,
}
impl ::pyroduct::format::value::TypeableRow for Outer {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            {
                                let field = ::pyroduct::format::value::PyroField::<
                                    'static,
                                >::new(
                                    "inner",
                                    <Inner as ::pyroduct::format::value::Typeable>::pyro_type(),
                                    <Inner as ::pyroduct::format::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let field = ::pyroduct::format::value::PyroField::<
                                    'static,
                                >::new(
                                    "count",
                                    <u32 as ::pyroduct::format::value::Typeable>::pyro_type(),
                                    <u32 as ::pyroduct::format::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                        ],
                    ),
                ),
            ),
            documentation: Some(::std::borrow::Cow::Borrowed("Outer struct")),
        }
    }
}
fn main() {}
