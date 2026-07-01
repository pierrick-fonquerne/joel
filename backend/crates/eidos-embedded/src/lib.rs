//! Embedded `EidosDB` server: open a data directory and serve gRPC in-process.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use eidosdb_proto::pb::eidos_db_server::EidosDbServer;
use eidosdb_server::registry::Registry;
use eidosdb_server::service::EidosDbService;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;

/// Failure modes while starting the embedded server.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddedError {
    /// Opening the data directory failed.
    #[error("registry open failed: {0}")]
    Registry(String),
    /// Binding the listen address failed.
    #[error("bind failed: {0}")]
    Bind(String),
}

/// Handle that aborts the server task when dropped.
pub struct ShutdownHandle(JoinHandle<()>);

impl ShutdownHandle {
    /// Aborts the embedded server task immediately.
    pub fn shutdown(self) {
        self.0.abort();
    }
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Starts an embedded `EidosDB` server backed by a local data directory.
pub struct EmbeddedEidos;

impl EmbeddedEidos {
    /// Opens `data_dir` and serves gRPC on `addr`.
    ///
    /// Pass port `0` to let the OS assign an ephemeral port.
    ///
    /// Returns the reachable endpoint (`http://host:port`) and a shutdown
    /// handle.  Dropping the handle aborts the server task.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddedError`] if the data directory cannot be opened or
    /// the address cannot be bound.
    pub async fn start(
        data_dir: PathBuf,
        addr: SocketAddr,
    ) -> Result<(String, ShutdownHandle), EmbeddedError> {
        let reg = Registry::open(data_dir).map_err(|e| EmbeddedError::Registry(e.to_string()))?;
        let service = EidosDbServer::new(EidosDbService::new(Arc::new(reg)));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| EmbeddedError::Bind(e.to_string()))?;
        let local = listener
            .local_addr()
            .map_err(|e| EmbeddedError::Bind(e.to_string()))?;
        let task = tokio::spawn(async move {
            let _ = tonic::transport::Server::builder()
                .add_service(service)
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await;
        });
        Ok((format!("http://{local}"), ShutdownHandle(task)))
    }
}
