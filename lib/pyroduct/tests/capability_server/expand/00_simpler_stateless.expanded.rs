pub struct SimpleClient {
    #[rkyv(with = rkyv::with::Skip)]
    __config_buf: Vec<u8>,
}
#[automatically_derived]
///An archived [`SimpleClient`]
#[bytecheck(crate = ::rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedSimpleClient
where
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
{
    ///The archived counterpart of [`SimpleClient::__config_buf`]
    __config_buf: <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Archived,
}
#[automatically_derived]
unsafe impl<
    __C: ::rkyv::bytecheck::rancor::Fallible + ?::core::marker::Sized,
> ::rkyv::bytecheck::CheckBytes<__C> for ArchivedSimpleClient
where
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
    <__C as ::rkyv::bytecheck::rancor::Fallible>::Error: ::rkyv::bytecheck::rancor::Trace,
    <rkyv::with::Skip as ::rkyv::with::ArchiveWith<
        Vec<u8>,
    >>::Archived: ::rkyv::bytecheck::CheckBytes<__C>,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut __C,
    ) -> ::core::result::Result<
        (),
        <__C as ::rkyv::bytecheck::rancor::Fallible>::Error,
    > {
        <<rkyv::with::Skip as ::rkyv::with::ArchiveWith<
            Vec<u8>,
        >>::Archived as ::rkyv::bytecheck::CheckBytes<
            __C,
        >>::check_bytes(&raw const (*value).__config_buf, context)
            .map_err(|e| {
                <<__C as ::rkyv::bytecheck::rancor::Fallible>::Error as ::rkyv::bytecheck::rancor::Trace>::trace(
                    e,
                    ::rkyv::bytecheck::StructCheckContext {
                        struct_name: "ArchivedSimpleClient",
                        field_name: "__config_buf",
                    },
                )
            })?;
        ::core::result::Result::Ok(())
    }
}
#[automatically_derived]
///The resolver for an archived [`SimpleClient`]
pub struct SimpleClientResolver
where
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
{
    __config_buf: <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Resolver,
}
impl ::rkyv::Archive for SimpleClient
where
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
{
    type Archived = ArchivedSimpleClient;
    type Resolver = SimpleClientResolver;
    #[allow(clippy::unit_arg)]
    fn resolve(&self, resolver: Self::Resolver, out: ::rkyv::Place<Self::Archived>) {
        let field_ptr = unsafe { &raw mut (*out.ptr()).__config_buf };
        let field_out = unsafe { ::rkyv::Place::from_field_unchecked(out, field_ptr) };
        <rkyv::with::Skip as ::rkyv::with::ArchiveWith<
            Vec<u8>,
        >>::resolve_with(&self.__config_buf, resolver.__config_buf, field_out);
    }
}
unsafe impl ::rkyv::traits::Portable for ArchivedSimpleClient
where
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
    <rkyv::with::Skip as ::rkyv::with::ArchiveWith<
        Vec<u8>,
    >>::Archived: ::rkyv::traits::Portable,
{}
#[automatically_derived]
impl<__S: ::rkyv::rancor::Fallible + ?Sized> ::rkyv::Serialize<__S> for SimpleClient
where
    rkyv::with::Skip: ::rkyv::with::SerializeWith<Vec<u8>, __S>,
{
    fn serialize(
        &self,
        serializer: &mut __S,
    ) -> ::core::result::Result<
        <Self as ::rkyv::Archive>::Resolver,
        <__S as ::rkyv::rancor::Fallible>::Error,
    > {
        let __this = self;
        ::core::result::Result::Ok(SimpleClientResolver {
            __config_buf: <rkyv::with::Skip as ::rkyv::with::SerializeWith<
                Vec<u8>,
                __S,
            >>::serialize_with(&__this.__config_buf, serializer)?,
        })
    }
}
#[automatically_derived]
impl<__D: ::rkyv::rancor::Fallible + ?Sized> ::rkyv::Deserialize<SimpleClient, __D>
for ::rkyv::Archived<SimpleClient>
where
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
    rkyv::with::Skip: ::rkyv::with::DeserializeWith<
        <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Archived,
        Vec<u8>,
        __D,
    >,
{
    fn deserialize(
        &self,
        deserializer: &mut __D,
    ) -> ::core::result::Result<SimpleClient, <__D as ::rkyv::rancor::Fallible>::Error> {
        let __this = self;
        ::core::result::Result::Ok(SimpleClient {
            __config_buf: <rkyv::with::Skip as ::rkyv::with::DeserializeWith<
                <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Archived,
                Vec<u8>,
                __D,
            >>::deserialize_with(&__this.__config_buf, deserializer)?,
        })
    }
}
impl ::pyroduct::module_capability::CapabilityClient for SimpleClient {
    fn config_buffer(&self) -> &[u8] {
        &self.__config_buf
    }
}
impl SimpleClient {
    pub fn client() -> SimpleClient {
        let mut new_self = (|| {
            SimpleClient {
                __config_buf: std::vec::Vec::new(),
            }
        })();
        new_self.__config_buf = ::rkyv::to_bytes::<::rkyv::rancor::Error>(&new_self)
            .expect("Failed to serialize config")
            .into_vec();
        ::pyroduct::module_capability::access::call_from_wasm::<
            SimpleClient,
            (),
            (),
            _,
        >(
            "__simple__stateful_server__new_client",
            Some(&new_self),
            None,
            |
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize|
            {
                unsafe {
                    wasm::__simple__stateful_server__new_client__wasm(
                        client_state_ptr,
                        client_state_len,
                        input_ptr,
                        input_len,
                    )
                }
            },
        );
        new_self
    }
    pub fn call(&self) -> f32 {
        ::pyroduct::module_capability::access::call_from_wasm::<
            SimpleClient,
            (),
            f32,
            _,
        >(
            "__simple__stateful_server__call",
            Some(&self),
            None,
            |
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize|
            {
                unsafe {
                    wasm::__simple__stateful_server__call__wasm(
                        client_state_ptr,
                        client_state_len,
                        input_ptr,
                        input_len,
                    )
                }
            },
        )
    }
}
pub mod wasm {
    use super::*;
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn __simple__stateful_server__new_client__wasm(
            cs_ptr: *const u8,
            cs_len: usize,
            in_ptr: *const u8,
            in_len: usize,
        ) -> *const u8;
        pub fn __simple__stateful_server__call__wasm(
            cs_ptr: *const u8,
            cs_len: usize,
            in_ptr: *const u8,
            in_len: usize,
        ) -> *const u8;
    }
}
pub trait Simple {
    fn new_client(&self, client: &SimpleClient) -> ();
    fn call(&self, client: &SimpleClient) -> f32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __simple__stateful_server__new_client__ffi(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    capability_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_call::sc_call::<
        StatefulServer,
        SimpleClient,
        (),
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        capability_state_ptr,
        |state, client| state.new_client(&client),
    )
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __simple__stateful_server__call__ffi(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    capability_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_call::sc_call::<
        StatefulServer,
        SimpleClient,
        f32,
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        capability_state_ptr,
        |state, client| state.call(&client),
    )
}
const __SIMPLE__STATEFUL_SERVER: &'static str = "__simple__stateful_server";
const __SIMPLE__STATEFUL_SERVER__NEW_CLIENT: &'static str = "__simple__stateful_server__new_client";
const __SIMPLE__STATEFUL_SERVER__CALL: &'static str = "__simple__stateful_server__call";
const __SIMPLE__STATEFUL_SERVER__METHODS: [::pyroduct::capability_host::ffi::FunctionExport; 2usize] = [
    ::pyroduct::capability_host::ffi::FunctionExport {
        module: __SIMPLE__STATEFUL_SERVER.as_ptr(),
        module_len: __SIMPLE__STATEFUL_SERVER.len(),
        name: __SIMPLE__STATEFUL_SERVER__NEW_CLIENT.as_ptr(),
        name_len: __SIMPLE__STATEFUL_SERVER__NEW_CLIENT.len(),
        func: ::pyroduct::capability_host::ffi::Function::Sync(
            __simple__stateful_server__new_client__ffi,
        ),
    },
    ::pyroduct::capability_host::ffi::FunctionExport {
        module: __SIMPLE__STATEFUL_SERVER.as_ptr(),
        module_len: __SIMPLE__STATEFUL_SERVER.len(),
        name: __SIMPLE__STATEFUL_SERVER__CALL.as_ptr(),
        name_len: __SIMPLE__STATEFUL_SERVER__CALL.len(),
        func: ::pyroduct::capability_host::ffi::Function::Sync(
            __simple__stateful_server__call__ffi,
        ),
    },
];
impl Simple for StatefulServer {
    fn new_client(&self, client: &SimpleClient) {}
    fn call(&self, client: &SimpleClient) -> f32 {
        42.0
    }
}
impl StatefulServerInit for StatefulServer {
    fn new(config: &()) -> Self {
        Self
    }
    fn default() -> Self {
        Self
    }
    fn reset(&mut self) {}
}
fn main() {}
