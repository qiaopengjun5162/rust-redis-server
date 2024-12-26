use anyhow::Result;
use rust_redis_server::{network, Backend};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let addr = "0.0.0.0:63791";
    info!("Redis server listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;

    let backend = Backend::new();
    loop {
        let (stream, raddr) = listener.accept().await?;
        info!("Accepted connection from {}", raddr);
        let cloned_backend = backend.clone();
        tokio::spawn(async move {
            if let Err(e) = network::stream_handler(stream, cloned_backend).await {
                warn!("Error handling connection from {}: {}", raddr, e);
            }
        });
    }
}
