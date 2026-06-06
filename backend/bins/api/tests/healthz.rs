//! Integration test for the health endpoint.
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "../../migrations")]
async fn healthz_reports_db_up(pool: PgPool) {
    let app = api::build_router(pool);

    let response = app
        .oneshot(Request::get("/api/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["db"], "up");
}
