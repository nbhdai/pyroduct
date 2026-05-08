use pyroduct::format::Document;
struct SimpleStruct {
    id: u32,
    name: String,
}
impl ::pyroduct::format::value::TypeableRow for SimpleStruct {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "id",
                                <u32 as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <u32 as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "name",
                                <String as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <String as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                    ]),
                ),
            ),
            documentation: None,
        }
    }
}
fn main() {}
