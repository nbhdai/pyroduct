use pyroduct::Document;
struct SimpleStruct {
    id: u32,
    name: String,
}
impl ::pyroduct::value::TypeableRow for SimpleStruct {
    fn schema() -> ::pyroduct::value::PyroSchema<'static> {
        ::pyroduct::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        {
                            let mut field = ::pyroduct::value::PyroField::<
                                'static,
                            >::new(
                                "id",
                                <u32 as ::pyroduct::value::Typeable>::pyro_type(),
                                <u32 as ::pyroduct::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let mut field = ::pyroduct::value::PyroField::<
                                'static,
                            >::new(
                                "name",
                                <String as ::pyroduct::value::Typeable>::pyro_type(),
                                <String as ::pyroduct::value::Typeable>::is_nullable(),
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
