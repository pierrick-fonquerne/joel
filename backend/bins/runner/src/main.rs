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
    let embedder = Arc::new(
        tokio::task::spawn_blocking(knowledge::CandleEmbedder::load)
            .await
            .map_err(|error| format!("embedder load task panicked: {error}"))??,
    );

    tracing::info!("bootstrapping EidosDB");
    let (_store, handle) = runner::bootstrap(data_dir.into(), addr, embedder).await?;

    let exchange_rates = match std::env::var("DATABASE_URL") {
        Ok(url) => match sqlx::PgPool::connect(&url).await {
            Ok(pool) => Some(persistence::wealth::PgExchangeRates::new(pool)),
            Err(error) => {
                tracing::error!(%error, "cannot connect to database, exchange rates disabled");
                None
            }
        },
        Err(_) => None,
    };
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let mut last_rate_refresh: Option<std::time::Instant> = None;

    tracing::info!("runner started");
    let mut interval = tokio::time::interval(Duration::from_mins(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let is_due = last_rate_refresh
                    .is_none_or(|at| at.elapsed() >= runner::exchange_rate_refresh::REFRESH_INTERVAL);
                if let (true, Some(store)) = (is_due, exchange_rates.as_ref()) {
                    match runner::exchange_rate_refresh::refresh(&http, store).await {
                        Ok(inserted) => {
                            last_rate_refresh = Some(std::time::Instant::now());
                            tracing::info!(inserted, "exchange rates refreshed");
                        }
                        Err(error) => tracing::warn!(%error, "exchange rate refresh failed"),
                    }
                }
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
