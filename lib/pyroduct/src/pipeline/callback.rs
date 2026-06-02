use crate::format::value::PyroRow;

#[cfg(all(feature = "host", feature = "transport"))]
use crate::transport::socket::playbook::PlaybookClient;

use tracing::{debug, error};

pub enum Callback {
    #[cfg(all(feature = "host", feature = "transport"))]
    Socket(PlaybookClient),
    Function(fn(usize, &PyroRow<'_>)),
    #[cfg(all(feature = "host", feature = "transport"))]
    Http {
        client: reqwest::Client,
        url: String,
    },
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(all(feature = "host", feature = "transport"))]
            Callback::Socket(_) => f.debug_tuple("Socket").finish(),
            Callback::Function(func) => f
                .debug_tuple("Function")
                .field(&(func as *const _))
                .finish(),
            #[cfg(all(feature = "host", feature = "transport"))]
            Callback::Http { url, .. } => f.debug_struct("Http").field("url", url).finish(),
        }
    }
}

impl Callback {
    /// Create a function callback.
    pub fn function(f: fn(usize, &PyroRow<'_>)) -> Self {
        Callback::Function(f)
    }

    /// Create a socket callback with an existing PlaybookClient.
    #[cfg(all(feature = "host", feature = "transport"))]
    pub fn socket(client: PlaybookClient) -> Self {
        Callback::Socket(client)
    }

    /// Connect to a Unix domain socket and return a Socket callback.
    #[cfg(all(feature = "host", feature = "transport"))]
    pub async fn connect_socket_unix(
        path: impl AsRef<std::path::Path> + std::fmt::Debug,
    ) -> Result<Self, crate::PyroError> {
        let client = PlaybookClient::connect_unix(path).await?;
        Ok(Callback::Socket(client))
    }

    /// Connect to a TCP socket and return a Socket callback.
    #[cfg(all(feature = "host", feature = "transport"))]
    pub async fn connect_socket_tcp(
        addr: impl tokio::net::ToSocketAddrs + std::fmt::Debug,
    ) -> Result<Self, crate::PyroError> {
        let client = PlaybookClient::connect_tcp(addr).await?;
        Ok(Callback::Socket(client))
    }

    /// Create an HTTP callback with a default reqwest Client.
    #[cfg(all(feature = "host", feature = "transport"))]
    pub fn http(url: impl Into<String>) -> Self {
        Callback::Http {
            client: reqwest::Client::new(),
            url: url.into(),
        }
    }

    /// Create an HTTP callback with a custom pre-configured reqwest Client.
    #[cfg(all(feature = "host", feature = "transport"))]
    pub fn http_with_client(client: reqwest::Client, url: impl Into<String>) -> Self {
        Callback::Http {
            client,
            url: url.into(),
        }
    }

    pub async fn execute(&mut self, row_index: usize, input: &PyroRow<'_>) {
        match self {
            Callback::Function(f) => {
                f(row_index, input);
            }
            #[cfg(all(feature = "host", feature = "transport"))]
            Callback::Socket(client) => {
                debug!(row_index, "Executing socket callback using PlaybookClient");
                let _ = client.call(input).await;
            }
            #[cfg(all(feature = "host", feature = "transport"))]
            Callback::Http { client, url } => {
                debug!(
                    row_index,
                    url, "Executing HTTP callback targeting PlaybookHttpServer compatibility"
                );
                #[cfg(feature = "host")]
                {
                    let json_val = match serde_json::to_value(input) {
                        Ok(val) => val,
                        Err(e) => {
                            error!(
                                "Failed to serialize row to JSON value for HTTP callback: {:?}",
                                e
                            );
                            return;
                        }
                    };

                    match client
                        .post(url.as_str())
                        .header("X-Row-Index", row_index.to_string())
                        .json(&json_val)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                error!(
                                    "HTTP callback returned non-success status: {:?}",
                                    resp.status()
                                );
                            }
                        }
                        Err(e) => {
                            error!("HTTP callback failed: {:?}", e);
                        }
                    }
                }
            }
        }
    }
}
