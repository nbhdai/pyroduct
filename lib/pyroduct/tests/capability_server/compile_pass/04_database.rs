use pyroduct::{capability, capability_client, capability_impl};

#[capability_client]
#[derive(Clone)]
pub struct DbClient {
    url: String,
}

#[capability]
pub trait Database {
    async fn query(
        #[client_state] client: &DbClient, 
        query: String
    ) -> Result<Vec<String>, String>;
}

pub struct DbServer;

#[capability_impl(env = "database")]
impl Database for DbServer {
    async fn query(client: &DbClient, query: String) -> Result<Vec<String>, String> {
        Ok(vec![client.url.clone(), query])
    }
}

fn main() {
    // Just verifying compilation
}