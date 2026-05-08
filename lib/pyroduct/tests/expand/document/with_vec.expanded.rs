use pyroduct::format::Document;
struct DataContainer {
    items: Vec<u8>,
    labels: Vec<String>,
}
impl ::pyroduct::format::value::TypeableRow for DataContainer {
    fn schema() -> ::pyroduct::format::value::PyroSchema<'static> {
        ::pyroduct::format::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "items",
                                <Vec<
                                    u8,
                                > as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <Vec<
                                    u8,
                                > as ::pyroduct::format::value::Typeable>::is_nullable(),
                            );
                            field
                        },
                        {
                            let field = ::pyroduct::format::value::PyroField::<
                                'static,
                            >::new(
                                "labels",
                                <Vec<
                                    String,
                                > as ::pyroduct::format::value::Typeable>::pyro_type(),
                                <Vec<
                                    String,
                                > as ::pyroduct::format::value::Typeable>::is_nullable(),
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
