use pyroduct::{capability, capability_server, capability_impl};

#[capability]
pub trait Greeter {
    fn greet(&self, name: String) -> String;
}

pub struct GreeterServer;

#[capability_impl(env = "greeter")]
impl Greeter for GreeterServer {
    fn greet(&self, name: String) -> String {
        format!("Hello, {}", name)
    }
}

fn main() {
    // Just verifying compilation
}