//! HTTP adapters for passkey registration and discoverable login.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use domain::auth::ports::CredentialRepository;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;
use webauthn_rs::prelude::{
    DiscoverableKey, Passkey, PublicKeyCredential, RegisterPublicKeyCredential,
};

use crate::auth_routes::{CurrentUser, session_cookie};
use crate::state::AppState;

/// Request body completing a passkey registration.
#[derive(Deserialize)]
pub struct RegisterFinish {
    /// Human-readable device label ("iPhone", "PC").
    pub label: String,
    /// Credential produced by the browser.
    pub credential: RegisterPublicKeyCredential,
}

/// Request body completing a discoverable login.
#[derive(Deserialize)]
pub struct LoginFinish {
    /// Challenge identifier returned by the start endpoint.
    pub challenge_id: Uuid,
    /// Assertion produced by the browser.
    pub credential: PublicKeyCredential,
}

async fn register_start(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Response {
    let existing: Vec<Passkey> = match state.credentials.for_user(user.id).await {
        Ok(raw) => raw
            .iter()
            .filter_map(|j| serde_json::from_str(j).ok())
            .collect(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let exclude = existing
        .iter()
        .map(|p| p.cred_id().clone())
        .collect::<Vec<_>>();
    match state.webauthn.start_passkey_registration(
        user.id,
        &user.email,
        &user.display_name,
        Some(exclude),
    ) {
        Ok((ccr, reg_state)) => {
            state.reg_states.lock().await.insert(user.id, reg_state);
            Json(ccr).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn register_finish(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<RegisterFinish>,
) -> Response {
    let Some(reg_state) = state.reg_states.lock().await.remove(&user.id) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(passkey) = state
        .webauthn
        .finish_passkey_registration(&body.credential, &reg_state)
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(serialized) = serde_json::to_string(&passkey) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match state
        .credentials
        .insert(user.id, &body.label, &serialized)
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn login_start(State(state): State<AppState>) -> Response {
    match state.webauthn.start_discoverable_authentication() {
        Ok((rcr, auth_state)) => {
            let challenge_id = Uuid::new_v4();
            state
                .auth_states
                .lock()
                .await
                .insert(challenge_id, auth_state);
            Json(json!({ "challenge_id": challenge_id, "options": rcr })).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn login_finish(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginFinish>,
) -> Response {
    let Some(auth_state) = state.auth_states.lock().await.remove(&body.challenge_id) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok((user_id, _cred_id)) = state
        .webauthn
        .identify_discoverable_authentication(&body.credential)
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let keys: Vec<DiscoverableKey> = match state.credentials.for_user(user_id).await {
        Ok(raw) => raw
            .iter()
            .filter_map(|j| serde_json::from_str::<Passkey>(j).ok())
            .map(|p| DiscoverableKey::from(&p))
            .collect(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if keys.is_empty() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if state
        .webauthn
        .finish_discoverable_authentication(&body.credential, auth_state, &keys)
        .is_err()
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match state
        .auth
        .issue_session(user_id, OffsetDateTime::now_utc())
        .await
    {
        Ok(issued) => (jar.add(session_cookie(&issued)), StatusCode::OK).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Routes under `/api/auth/webauthn`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/webauthn/register/start", post(register_start))
        .route("/api/auth/webauthn/register/finish", post(register_finish))
        .route("/api/auth/webauthn/login/start", post(login_start))
        .route("/api/auth/webauthn/login/finish", post(login_finish))
}
