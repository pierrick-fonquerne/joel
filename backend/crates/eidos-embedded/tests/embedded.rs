//! Integration test: spin up an embedded `EidosDB` server and exercise it via
//! the typed gRPC client.

#![allow(clippy::expect_used)]

use std::net::SocketAddr;

use eidos_embedded::EmbeddedEidos;
use eidosdb_client::{CollectionSpec, EidosClient};
use eidosdb_core::{Dimension, Metric};
use eidosdb_hnsw::HnswConfig;
use eidosdb_proto::convert::IndexTypeChoice;

#[tokio::test]
async fn starts_and_serves_a_reachable_server() {
    let dir = tempfile::tempdir().expect("tempdir");
    let addr: SocketAddr = "127.0.0.1:0".parse().expect("addr");
    let (endpoint, _handle) = EmbeddedEidos::start(dir.path().to_path_buf(), addr)
        .await
        .expect("start");

    let mut client = EidosClient::connect(endpoint).await.expect("connect");
    client
        .create_collection(CollectionSpec {
            name: "press".into(),
            metric: Metric::Cosine,
            dimension: Dimension(3),
            index_type: IndexTypeChoice::Hnsw,
            hnsw: Some(HnswConfig::default()),
        })
        .await
        .expect("create");
    assert_eq!(client.list_collections().await.expect("list").len(), 1);
}
