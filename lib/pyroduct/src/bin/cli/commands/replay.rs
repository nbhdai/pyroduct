use anyhow::{Context, Result};
use fs_err as fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UnixStream};

pub async fn replay(input_file: &Path, socket_addr: &str) -> Result<()> {
    tracing::info!("Replaying {:?} to socket {}", input_file, socket_addr);

    let file = fs::File::open(input_file).context("Failed to open input file")?;
    let reader = BufReader::new(file);

    let mut count = 0;
    for line in reader.lines() {
        let line = line.context("Failed to read line from input file")?;
        if line.trim().is_empty() {
            continue;
        }

        let mut stream: Box<dyn tokio::io::AsyncWrite + Unpin + Send> =
            if let Ok(addr) = socket_addr.parse::<std::net::SocketAddr>() {
                Box::new(
                    TcpStream::connect(addr)
                        .await
                        .with_context(|| format!("Failed to connect to TCP socket {}", addr))?,
                )
            } else {
                Box::new(
                    UnixStream::connect(Path::new(socket_addr))
                        .await
                        .with_context(|| {
                            format!("Failed to connect to Unix socket {:?}", socket_addr)
                        })?,
                )
            };

        stream
            .write_all(line.as_bytes())
            .await
            .context("Failed to write to socket")?;
        stream
            .write_all(b"\n")
            .await
            .context("Failed to write newline to socket")?;

        count += 1;
    }

    tracing::info!("Successfully replayed {} rows", count);
    Ok(())
}
