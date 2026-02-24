use pyroduct::Document;
struct SimpleStruct {
    id: u32,
    name: String,
}
impl ::pyroduct::value::TypeableRow for SimpleStruct {
    fn schema() -> ::pyroduct::value::PyroSchema<'static> {
        ::pyroduct::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            {
                                let field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "id",
                                    <u32 as ::pyroduct::value::Typeable>::pyro_type(),
                                    <u32 as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "name",
                                    <String as ::pyroduct::value::Typeable>::pyro_type(),
                                    <String as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                        ],
                    ),
                ),
            ),
            documentation: None,
        }
    }
}
fn main() {}
