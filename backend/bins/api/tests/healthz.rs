//! Integration test for the health endpoint.
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

fn test_config() -> api::Config {
    api::Config {
        master_key_b64: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".to_owned(),
        webauthn_rp_id: "localhost".to_owned(),
        webauthn_origin: "http://localhost:4200".to_owned(),
        session_ttl_days: 30,
    }
}

#[tokio::test]
async fn healthz_reports_degraded_with_503_when_db_unreachable() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://joel:wrong@127.0.0.1:59999/joel")
        .unwrap();
    let app = api::build_router_with(pool, &test_config()).unwrap();

    let response = app
        .oneshot(Request::get("/api/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "degraded");
    assert_eq!(json["db"], "down");
}

#[sqlx::test(migrations = "../../migrations")]
async fn healthz_reports_db_up(pool: PgPool) {
    let app = api::build_router_with(pool, &test_config()).unwrap();

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

#[sqlx::test(migrations = "../../migrations")]
async fn openapi_spec_is_served(pool: PgPool) {
    let app = api::build_router_with(pool, &test_config()).unwrap();
    let response = app
        .oneshot(
            Request::get("/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["info"]["title"], "Joel API");
    assert!(json["paths"]["/api/healthz"].is_object());
}
