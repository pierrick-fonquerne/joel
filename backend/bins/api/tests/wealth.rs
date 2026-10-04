//! Integration tests of the wealth routes, with a fake cipher behind the gate.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

use std::sync::Arc;
use std::time::Duration;

use api::wealth_vault::{SealedUnlocker, VaultUnlocker, WealthGate};
use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use domain::auth::crypto::{PasswordService, SecretBox, TotpService};
use domain::auth::model::User;
use domain::auth::ports::UserRepository;
use domain::wealth::FieldCipher;
use domain::wealth::test_support::FakeCipher;
use http_body_util::BodyExt;
use persistence::auth::PgUsers;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const KEY_B64: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";

struct FakeUnlocker;

#[async_trait]
impl VaultUnlocker for FakeUnlocker {
    async fn unlock(&self) -> Result<Arc<dyn FieldCipher>, String> {
        Ok(Arc::new(FakeCipher))
    }
}

fn config() -> api::Config {
    api::Config {
        master_key_b64: KEY_B64.to_owned(),
        webauthn_rp_id: "localhost".to_owned(),
        webauthn_origin: "http://localhost:4200".to_owned(),
        session_ttl_days: 30,
        egide_url: None,
        egide_token: None,
    }
}

async fn app_and_cookie(pool: PgPool, unlocker: Arc<dyn VaultUnlocker>) -> (Router, String) {
    let secret = TotpService::generate_secret();
    PgUsers::new(pool.clone())
        .insert(&User {
            id: Uuid::new_v4(),
            email: "pierrick@example.com".into(),
            display_name: "Pierrick".into(),
            password_hash: PasswordService::hash("hunter2hunter2").unwrap(),
            totp_secret_enc: SecretBox::from_base64(KEY_B64).unwrap().seal(&secret),
        })
        .await
        .unwrap();
    let gate = Arc::new(WealthGate::new(pool.clone(), unlocker, Duration::ZERO));
    let app = api::build_router_with_gate(pool, &config(), gate).unwrap();
    let now = u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap();
    let code = TotpService::current_code(&secret, now).unwrap();
    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "pierrick@example.com", "password": "hunter2hunter2", "totp": code }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    (app, cookie)
}

async fn call(
    app: &Router,
    cookie: &str,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn today() -> String {
    time::OffsetDateTime::now_utc().date().to_string()
}

#[sqlx::test(migrations = "../../migrations")]
async fn full_flow_create_value_and_read_net_worth(pool: PgPool) {
    let (app, cookie) = app_and_cookie(pool, Arc::new(FakeUnlocker)).await;

    let (status, pea) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "PEA Bourso", "kind": "brokerage_pea", "owner": "personal", "currency": "EUR" }))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, loan) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "Prêt maison", "kind": "loan", "owner": "personal", "currency": "EUR" }))).await;

    let pea_id = pea["id"].as_str().unwrap();
    let loan_id = loan["id"].as_str().unwrap();
    let (status, valuation) = call(
        &app,
        &cookie,
        "POST",
        &format!("/api/wealth/accounts/{pea_id}/valuations"),
        Some(json!({ "amount": "50000.50", "as_of": today() })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(valuation["amount"], "50000.50");
    call(
        &app,
        &cookie,
        "POST",
        &format!("/api/wealth/accounts/{loan_id}/valuations"),
        Some(json!({ "amount": "-20000", "as_of": today() })),
    )
    .await;

    let (status, net_worth) = call(&app, &cookie, "GET", "/api/wealth/net-worth", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(net_worth["total"], "30000.50");
    assert_eq!(net_worth["by_owner"]["personal"], "30000.50");
    assert_eq!(net_worth["by_kind"]["loan"], "-20000");

    let (status, accounts) = call(&app, &cookie, "GET", "/api/wealth/accounts", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(accounts[0]["name"], "PEA Bourso");
    assert_eq!(accounts[0]["is_stale"], false);
    assert_eq!(accounts[0]["latest_valuation"]["amount"], "50000.50");

    let (status, history) = call(
        &app,
        &cookie,
        "GET",
        &format!("/api/wealth/accounts/{pea_id}/valuations"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(history.as_array().unwrap().len(), 1);

    let (status, points) = call(
        &app,
        &cookie,
        "GET",
        &format!(
            "/api/wealth/net-worth/history?from=2026-01-01&to={}",
            today()
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        points.as_array().unwrap().last().unwrap()["total"],
        "30000.50"
    );

    let (status, _) = call(
        &app,
        &cookie,
        "POST",
        &format!("/api/wealth/accounts/{pea_id}/archive"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn validation_errors_carry_business_codes(pool: PgPool) {
    let (app, cookie) = app_and_cookie(pool, Arc::new(FakeUnlocker)).await;
    let (_, account) = call(
        &app,
        &cookie,
        "POST",
        "/api/wealth/accounts",
        Some(
            json!({ "name": "Livret", "kind": "savings", "owner": "personal", "currency": "EUR" }),
        ),
    )
    .await;
    let id = account["id"].as_str().unwrap();

    let cases = [
        (
            json!({ "name": "X", "kind": "savings", "owner": "personal", "currency": "BTC" }),
            "/api/wealth/accounts".to_owned(),
            "unsupported_currency",
        ),
        (
            json!({ "name": "X", "kind": "stocks", "owner": "personal", "currency": "EUR" }),
            "/api/wealth/accounts".to_owned(),
            "invalid_account_kind",
        ),
        (
            json!({ "amount": "1,5", "as_of": today() }),
            format!("/api/wealth/accounts/{id}/valuations"),
            "invalid_amount",
        ),
        (
            json!({ "amount": "-5", "as_of": today() }),
            format!("/api/wealth/accounts/{id}/valuations"),
            "negative_asset_valuation",
        ),
        (
            json!({ "amount": "5", "as_of": "2999-01-01" }),
            format!("/api/wealth/accounts/{id}/valuations"),
            "future_valuation",
        ),
        (
            json!({ "amount": "5", "as_of": "04/10/2026" }),
            format!("/api/wealth/accounts/{id}/valuations"),
            "invalid_date",
        ),
    ];
    for (body, uri, code) in cases {
        let (status, error) = call(&app, &cookie, "POST", &uri, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{code}");
        assert_eq!(error["code"], code);
    }

    let (status, error) = call(
        &app,
        &cookie,
        "GET",
        &format!(
            "/api/wealth/net-worth/history?from=2000-01-01&to={}",
            today()
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["code"], "invalid_range");

    let unknown = Uuid::new_v4();
    let (status, error) = call(
        &app,
        &cookie,
        "POST",
        &format!("/api/wealth/accounts/{unknown}/valuations"),
        Some(json!({ "amount": "5", "as_of": today() })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["code"], "account_not_found");
}

#[sqlx::test(migrations = "../../migrations")]
async fn sealed_vault_answers_503_and_anonymous_answers_401(pool: PgPool) {
    let (app, cookie) = app_and_cookie(pool, Arc::new(SealedUnlocker)).await;
    let (status, error) = call(&app, &cookie, "GET", "/api/wealth/accounts", None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error["code"], "wealth_vault_sealed");

    let (status, _) = call(
        &app,
        "joel_session=nope",
        "GET",
        "/api/wealth/net-worth",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_exchange_rate_answers_422(pool: PgPool) {
    sqlx::query("INSERT INTO wealth_exchange_rates (currency, on_date, units_per_eur) VALUES ('USD', '2026-01-02', 1.1)")
        .execute(&pool)
        .await
        .unwrap();
    let (app, cookie) = app_and_cookie(pool, Arc::new(FakeUnlocker)).await;
    let (_, account) = call(&app, &cookie, "POST", "/api/wealth/accounts",
        Some(json!({ "name": "Broker US", "kind": "brokerage_cto", "owner": "personal", "currency": "USD" }))).await;
    let id = account["id"].as_str().unwrap();
    call(
        &app,
        &cookie,
        "POST",
        &format!("/api/wealth/accounts/{id}/valuations"),
        Some(json!({ "amount": "100", "as_of": today() })),
    )
    .await;

    let (status, error) = call(&app, &cookie, "GET", "/api/wealth/net-worth", None).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["code"], "exchange_rate_missing");
}
