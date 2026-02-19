use pyroduct::Document;
struct DataContainer {
    items: Vec<u8>,
    labels: Vec<String>,
}
impl ::pyroduct::value::TypeableRow for DataContainer {
    fn schema() -> ::pyroduct::value::PyroSchema<'static> {
        ::pyroduct::value::PyroSchema {
            fields: ::std::borrow::Cow::Owned(
                ::alloc::boxed::box_assume_init_into_vec_unsafe(
                    ::alloc::intrinsics::write_box_via_move(
                        ::alloc::boxed::Box::new_uninit(),
                        [
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "items",
                                    <Vec<u8> as ::pyroduct::value::Typeable>::pyro_type(),
                                    <Vec<u8> as ::pyroduct::value::Typeable>::is_nullable(),
                                );
                                field
                            },
                            {
                                let mut field = ::pyroduct::value::PyroField::<
                                    'static,
                                >::new(
                                    "labels",
                                    <Vec<String> as ::pyroduct::value::Typeable>::pyro_type(),
                                    <Vec<String> as ::pyroduct::value::Typeable>::is_nullable(),
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
