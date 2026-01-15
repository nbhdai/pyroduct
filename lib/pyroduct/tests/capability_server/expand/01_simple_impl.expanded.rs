use pyroduct::{capability, capability_server, capability_impl};
pub trait Greeter {
    fn greet(&self, name: String) -> String;
}
pub struct GreeterServer;
impl Greeter for GreeterServer {
    fn greet(&self, name: String) -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("Hello, {0}", name))
        })
    }
}
impl GreeterServer {
    #[allow(clippy::needless_lifetimes)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_greet(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiResult {
        ::pyroduct::capability::safe_call::si_call::<
            GreeterServer,
            String,
            String,
            _,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |state, input| state.greet(input),
        )
    }
    pub fn __capability_exports() -> Vec<
        ::pyroduct::capability_host::ffi::PluginExport,
    > {
        static MOD_NAME: &str = "greeter";
        <[_]>::into_vec(
            ::alloc::boxed::box_new([
                ::pyroduct::capability_host::ffi::PluginExport {
                    module: MOD_NAME.as_ptr(),
                    module_len: MOD_NAME.len(),
                    name: "host_greet".as_ptr(),
                    name_len: "host_greet".len(),
                    func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(
                        host_greet,
                    ),
                },
            ]),
        )
    }
}
