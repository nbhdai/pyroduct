use pyroduct::{capability, capability_server, capability_impl};
pub struct GreeterServer;
impl Greeter for GreeterServer {
    fn greet(&self, name: String) -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("Hello, {0}", name))
        })
    }
}
