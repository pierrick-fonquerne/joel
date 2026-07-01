//! Joel runner: starts the embedded `EidosDB` server and runs the ingestion scheduler.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let data_dir = std::env::var("JOEL_EIDOS_DATA_DIR").unwrap_or_else(|_| "./data/eidos".into());
    let addr_str = std::env::var("JOEL_EIDOS_ADDR").unwrap_or_else(|_| "127.0.0.1:50100".into());
    let addr: SocketAddr = addr_str.parse()?;
    let embedder = Arc::new(knowledge::CandleEmbedder::load()?);

    tracing::info!("bootstrapping EidosDB");
    let (_store, handle) = runner::bootstrap(data_dir.into(), addr, embedder).await?;

    tracing::info!("runner started");
    let mut interval = tokio::time::interval(Duration::from_mins(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                tracing::info!("runner tick: no scheduled work yet");
            }
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    tracing::error!(%error, "signal handler failure");
                }
                tracing::info!("runner shutting down");
                break;
            }
        }
    }

    handle.shutdown();
    Ok(())
}
