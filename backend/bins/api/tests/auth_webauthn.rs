//! Integration tests for the `WebAuthn` endpoints (challenge issuance and rejection paths).
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

#[path = "auth_password.rs"]
mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn register_start_requires_a_session(pool: PgPool) {
    let app = api::build_router_with(pool, &common::test_config()).unwrap();
    let response = app
        .oneshot(
            Request::post("/api/auth/webauthn/register/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_start_returns_a_challenge(pool: PgPool) {
    let app = api::build_router_with(pool, &common::test_config()).unwrap();
    let response = app
        .oneshot(
            Request::post("/api/auth/webauthn/login/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["challenge_id"].is_string());
    assert!(json["options"]["publicKey"]["challenge"].is_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_finish_with_unknown_challenge_is_rejected(pool: PgPool) {
    let app = api::build_router_with(pool, &common::test_config()).unwrap();
    let response = app
        .oneshot(
            Request::post("/api/auth/webauthn/login/finish")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"challenge_id":"00000000-0000-0000-0000-000000000000","credential":{"id":"x","rawId":"eA","response":{"authenticatorData":"eA","clientDataJSON":"eA","signature":"eA"},"type":"public-key"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
