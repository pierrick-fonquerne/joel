//! Integration tests for password + TOTP login and the session middleware.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc, clippy::must_use_candidate, missing_docs)]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use domain::auth::crypto::{PasswordService, SecretBox, TotpService};
use domain::auth::model::User;
use domain::auth::ports::UserRepository;
use http_body_util::BodyExt;
use persistence::auth::PgUsers;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const KEY_B64: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";

pub async fn seed(pool: &PgPool) -> Vec<u8> {
    let secret = TotpService::generate_secret();
    let secret_box = SecretBox::from_base64(KEY_B64).unwrap();
    PgUsers::new(pool.clone())
        .insert(&User {
            id: Uuid::new_v4(),
            email: "pierrick@example.com".into(),
            display_name: "Pierrick".into(),
            password_hash: PasswordService::hash("hunter2hunter2").unwrap(),
            totp_secret_enc: secret_box.seal(&secret),
        })
        .await
        .unwrap();
    secret
}

fn now_unix() -> u64 {
    u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap()
}

pub fn test_config() -> api::Config {
    api::Config {
        master_key_b64: KEY_B64.to_owned(),
        webauthn_rp_id: "localhost".to_owned(),
        webauthn_origin: "http://localhost:4200".to_owned(),
        session_ttl_days: 30,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_sets_cookie_and_me_returns_identity(pool: PgPool) {
    let secret = seed(&pool).await;
    let app = api::build_router_with(pool, &test_config()).unwrap();
    let code = TotpService::current_code(&secret, now_unix()).unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"pierrick@example.com","password":"hunter2hunter2","totp":"{code}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().to_owned();
    assert!(cookie.contains("joel_session="));
    assert!(cookie.contains("HttpOnly"));

    let me = app
        .oneshot(Request::get("/api/auth/me").header(header::COOKIE, cookie.split(';').next().unwrap()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let body = me.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["email"], "pierrick@example.com");
}

#[sqlx::test(migrations = "../../migrations")]
async fn bad_credentials_yield_401_and_me_requires_session(pool: PgPool) {
    let _secret = seed(&pool).await;
    let app = api::build_router_with(pool, &test_config()).unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email":"pierrick@example.com","password":"wrong","totp":"000000"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::UNAUTHORIZED);

    let me = app.oneshot(Request::get("/api/auth/me").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_invalidates_the_cookie(pool: PgPool) {
    let secret = seed(&pool).await;
    let app = api::build_router_with(pool, &test_config()).unwrap();
    let code = TotpService::current_code(&secret, now_unix()).unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"pierrick@example.com","password":"hunter2hunter2","totp":"{code}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().split(';').next().unwrap().to_owned();

    let logout = app
        .clone()
        .oneshot(Request::post("/api/auth/logout").header(header::COOKIE, &cookie).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let me = app.oneshot(Request::get("/api/auth/me").header(header::COOKIE, &cookie).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}
