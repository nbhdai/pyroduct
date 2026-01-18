use pyroduct::capability;

capability! {
    env = "greeter",

    #[capability]
    trait Greeter {
        type Client = GreeterClient;
        
        fn new() -> GreeterClient {
            GreeterClient {}
        }
        
        fn greet(name: String) -> String;
    }

    #[capability_client]
    pub struct GreeterClient {}

    #[capability_server(config = GreeterConfig)]
    pub struct GreeterServer;

    pub struct GreeterConfig;

    #[capability]
    impl Greeter for GreeterServer {
        type Client = GreeterClient;
        
        fn new_client(&self, _client: &GreeterClient) -> () {
            // Initialize client state on server if needed
        }
        
        fn greet(&self, _client: &GreeterClient, name: String) -> String {
            format!("Hello, {}", name)
        }
    }
}

impl GreeterServerInit for GreeterServer {
    fn new(_config: &GreeterConfig) -> Self {
        GreeterServer
    }
    
    fn reset(&mut self) {
        // Reset server state if needed
    }
}

fn main() {
    // Just verifying compilation
}