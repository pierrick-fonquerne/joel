//! Runner bootstrap: embed the `EidosDB` server, ensure collections, build the store.

#![allow(clippy::module_name_repetitions)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use domain::knowledge::Embedder;
use eidos_embedded::{EmbeddedEidos, ShutdownHandle};
use eidosdb_client::EidosClient;
use knowledge::EidosKnowledgeStore;

/// Failure modes while bootstrapping the runner.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// Embedded server failed to start.
    #[error("embedded server: {0}")]
    Embedded(String),
    /// Client could not connect to the server endpoint.
    #[error("client connect: {0}")]
    Connect(String),
    /// Collection bootstrap failed.
    #[error("ensure collections: {0}")]
    Ensure(String),
}

/// Starts the embedded server, ensures collections, returns the store and a shutdown handle.
///
/// # Errors
///
/// Returns [`BootstrapError`] if any startup step fails.
pub async fn bootstrap(
    data_dir: PathBuf,
    addr: SocketAddr,
    embedder: Arc<dyn Embedder>,
) -> Result<(EidosKnowledgeStore, ShutdownHandle), BootstrapError> {
    let (endpoint, handle) = EmbeddedEidos::start(data_dir, addr)
        .await
        .map_err(|e| BootstrapError::Embedded(e.to_string()))?;
    let client = EidosClient::connect(endpoint)
        .await
        .map_err(|e| BootstrapError::Connect(e.to_string()))?;
    let store = EidosKnowledgeStore::new(client, embedder);
    store
        .ensure_collections()
        .await
        .map_err(|e| BootstrapError::Ensure(e.to_string()))?;
    Ok((store, handle))
}
