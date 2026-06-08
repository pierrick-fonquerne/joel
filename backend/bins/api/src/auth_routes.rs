//! HTTP adapters for the authentication use cases.

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use domain::auth::model::{AuthError, IssuedSession, User};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::state::AppState;

/// Name of the HTTP-only session cookie set on successful authentication.
pub const SESSION_COOKIE: &str = "joel_session";

/// Request body of the password login endpoint.
#[derive(Deserialize)]
pub struct LoginRequest {
    /// Account email.
    pub email: String,
    /// Account password.
    pub password: String,
    /// Current 6-digit `TOTP` code.
    pub totp: String,
}

/// Public identity of the authenticated user.
#[derive(Serialize)]
pub struct Identity {
    /// Account email.
    pub email: String,
    /// Display name.
    pub display_name: String,
}

/// Authenticated user extractor: resolves the session cookie or rejects with 401.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned()).ok_or(StatusCode::UNAUTHORIZED)?;
        state
            .auth
            .validate_session(&token, OffsetDateTime::now_utc())
            .await
            .map(CurrentUser)
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }
}

fn auth_error_response(error: &AuthError) -> StatusCode {
    match error {
        AuthError::InvalidCredentials | AuthError::NotAuthenticated => StatusCode::UNAUTHORIZED,
        AuthError::Crypto | AuthError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Builds a session cookie from an issued session.
#[must_use]
pub fn session_cookie(issued: &IssuedSession) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, issued.token.clone());
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_path("/");
    cookie.set_expires(issued.expires_at);
    cookie
}

async fn login(State(state): State<AppState>, jar: CookieJar, Json(body): Json<LoginRequest>) -> Response {
    match state.auth.password_login(&body.email, &body.password, &body.totp, OffsetDateTime::now_utc()).await {
        Ok(issued) => (jar.add(session_cookie(&issued)), StatusCode::OK).into_response(),
        Err(error) => auth_error_response(&error).into_response(),
    }
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    if let Some(cookie) = jar.get(SESSION_COOKIE)
        && state.auth.logout(cookie.value()).await.is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    (jar.remove(Cookie::from(SESSION_COOKIE)), StatusCode::NO_CONTENT).into_response()
}

async fn me(CurrentUser(user): CurrentUser) -> Json<Identity> {
    Json(Identity { email: user.email, display_name: user.display_name })
}

/// Routes under `/api/auth`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
}
