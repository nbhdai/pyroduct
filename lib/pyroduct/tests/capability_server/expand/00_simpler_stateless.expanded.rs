#[rkyv(crate = ::pyroduct::rkyv)]
pub struct SimpleClient;
#[automatically_derived]
///An archived [`SimpleClient`]
#[bytecheck(crate = ::pyroduct::rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedSimpleClient;
#[automatically_derived]
unsafe impl<
    __C: ::pyroduct::rkyv::bytecheck::rancor::Fallible + ?::core::marker::Sized,
> ::pyroduct::rkyv::bytecheck::CheckBytes<__C> for ArchivedSimpleClient
where
    <__C as ::pyroduct::rkyv::bytecheck::rancor::Fallible>::Error: ::pyroduct::rkyv::bytecheck::rancor::Trace,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut __C,
    ) -> ::core::result::Result<
        (),
        <__C as ::pyroduct::rkyv::bytecheck::rancor::Fallible>::Error,
    > {
        ::core::result::Result::Ok(())
    }
}
#[automatically_derived]
///The resolver for an archived [`SimpleClient`]
pub struct SimpleClientResolver;
impl ::pyroduct::rkyv::Archive for SimpleClient {
    type Archived = ArchivedSimpleClient;
    type Resolver = SimpleClientResolver;
    const COPY_OPTIMIZATION: ::pyroduct::rkyv::traits::CopyOptimization<Self> = unsafe {
        ::pyroduct::rkyv::traits::CopyOptimization::enable_if(
            0 == ::core::mem::size_of::<SimpleClient>(),
        )
    };
    #[allow(clippy::unit_arg)]
    fn resolve(
        &self,
        resolver: Self::Resolver,
        out: ::pyroduct::rkyv::Place<Self::Archived>,
    ) {}
}
unsafe impl ::pyroduct::rkyv::traits::Portable for ArchivedSimpleClient {}
#[automatically_derived]
impl<__S: ::pyroduct::rkyv::rancor::Fallible + ?Sized> ::pyroduct::rkyv::Serialize<__S>
for SimpleClient {
    fn serialize(
        &self,
        serializer: &mut __S,
    ) -> ::core::result::Result<
        <Self as ::pyroduct::rkyv::Archive>::Resolver,
        <__S as ::pyroduct::rkyv::rancor::Fallible>::Error,
    > {
        let __this = self;
        ::core::result::Result::Ok(SimpleClientResolver)
    }
}
#[automatically_derived]
impl<
    __D: ::pyroduct::rkyv::rancor::Fallible + ?Sized,
> ::pyroduct::rkyv::Deserialize<SimpleClient, __D>
for ::pyroduct::rkyv::Archived<SimpleClient> {
    fn deserialize(
        &self,
        deserializer: &mut __D,
    ) -> ::core::result::Result<
        SimpleClient,
        <__D as ::pyroduct::rkyv::rancor::Fallible>::Error,
    > {
        let __this = self;
        ::core::result::Result::Ok(SimpleClient)
    }
}
pub struct StatefulServer;
impl StatefulServer {
    pub fn new() -> Self {
        Self
    }
    pub fn reset(&mut self) {}
    pub fn new_client(&self, client: &SimpleClient) {}
    pub fn call(&self, _client: &SimpleClient) -> f32 {
        42.0
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn p__stateful_server__ffi_init(
    config_ptr: *const u8,
    config_len: usize,
) -> ::pyroduct::capability_host::ffi::FfiInitResult {
    unsafe {
        ::pyroduct::capability::safe_lifecycle::execute_safe_init::<
            ::pyroduct::capability::safe_lifecycle::EmptyConfig,
            StatefulServer,
            _,
        >(config_ptr, config_len, |_| StatefulServer::new())
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn p__stateful_server__ffi_drop(state: *mut std::ffi::c_void) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state as *mut StatefulServer) });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn p__stateful_server__ffi_reset(
    state: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_lifecycle::execute_safe_reset::<
        StatefulServer,
        _,
    >(state, |state| state.reset())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn p__stateful_server__new_client__ffi(
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
pub unsafe extern "C" fn p__stateful_server__call__ffi(
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
const CAPABILITY_NAME_VERSION: &'static str = "pyroduct-tests:0.0.0";
const p__STATEFUL_SERVER: &'static str = "p__stateful_server";
const p__STATEFUL_SERVER__NEW_CLIENT: &'static str = "p__stateful_server__new_client__wasm";
const p__STATEFUL_SERVER__CALL: &'static str = "p__stateful_server__call__wasm";
const p__STATEFUL_SERVER__METHODS: [::pyroduct::capability_host::ffi::FunctionExport; 2usize] = [
    ::pyroduct::capability_host::ffi::FunctionExport {
        capability: CAPABILITY_NAME_VERSION.as_ptr(),
        capability_len: CAPABILITY_NAME_VERSION.len(),
        name: p__STATEFUL_SERVER__NEW_CLIENT.as_ptr(),
        name_len: p__STATEFUL_SERVER__NEW_CLIENT.len(),
        func: ::pyroduct::capability_host::ffi::Function::Sync(
            p__stateful_server__new_client__ffi,
        ),
    },
    ::pyroduct::capability_host::ffi::FunctionExport {
        capability: CAPABILITY_NAME_VERSION.as_ptr(),
        capability_len: CAPABILITY_NAME_VERSION.len(),
        name: p__STATEFUL_SERVER__CALL.as_ptr(),
        name_len: p__STATEFUL_SERVER__CALL.len(),
        func: ::pyroduct::capability_host::ffi::Function::Sync(
            p__stateful_server__call__ffi,
        ),
    },
];
#[unsafe(no_mangle)]
pub extern "C" fn capability_manifest<'a>(
    id: u64,
    log_callback: ::pyroduct::capability_host::ffi::LogCallback,
) -> ::pyroduct::capability_host::ffi::ClassExport<'a> {
    ::pyroduct::capability::init_logging(id, log_callback);
    ::pyroduct::capability_host::ffi::ClassExport {
        len: p__STATEFUL_SERVER__METHODS.len(),
        ptr: p__STATEFUL_SERVER__METHODS.as_ptr() as *mut _,
        init: ::pyroduct::capability_host::ffi::ClassInitFn::Sync(
            p__stateful_server__ffi_init,
        ),
        drop: ::pyroduct::capability_host::ffi::ClassDropFn::Sync(
            p__stateful_server__ffi_drop,
        ),
        reset: ::pyroduct::capability_host::ffi::ClassResetFn::Sync(
            p__stateful_server__ffi_reset,
        ),
    }
}
fn main() {}
