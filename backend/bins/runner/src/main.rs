//! Joel runner: scheduler and job executor (skeleton for brick 0).

use std::time::Duration;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

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
}
