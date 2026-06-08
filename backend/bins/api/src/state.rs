//! Shared application state and configuration.

use std::collections::HashMap;
use std::sync::Arc;

use domain::auth::crypto::SecretBox;
use domain::auth::model::AuthError;
use domain::auth::use_cases::Auth;
use persistence::auth::{PgAudit, PgCredentials, PgSessions, PgUsers};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, PasskeyRegistration, Url, Webauthn, WebauthnBuilder,
};

/// Runtime configuration, sourced from the environment.
pub struct Config {
    /// Base64-encoded 32-byte master key for secrets at rest.
    pub master_key_b64: String,
    /// [`WebAuthn`] relying-party id (the apex hostname).
    pub webauthn_rp_id: String,
    /// [`WebAuthn`] origin (scheme + host + port presented by the browser).
    pub webauthn_origin: String,
    /// Session lifetime in days.
    pub session_ttl_days: i64,
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// # Errors
    /// Returns the name of the missing variable.
    pub fn from_env() -> Result<Self, String> {
        let need = |name: &str| std::env::var(name).map_err(|_| name.to_owned());
        Ok(Self {
            master_key_b64: need("MASTER_KEY")?,
            webauthn_rp_id: need("WEBAUTHN_RP_ID")?,
            webauthn_origin: need("WEBAUTHN_ORIGIN")?,
            session_ttl_days: need("SESSION_TTL_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        })
    }
}

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database pool (health checks and future modules).
    pub pool: PgPool,
    /// Authentication use cases.
    pub auth: Arc<Auth>,
    /// [`WebAuthn`] engine.
    pub webauthn: Arc<Webauthn>,
    /// Passkey storage adapter.
    pub credentials: Arc<PgCredentials>,
    /// In-flight passkey registration states, keyed by user id.
    pub reg_states: Arc<Mutex<HashMap<Uuid, PasskeyRegistration>>>,
    /// In-flight discoverable authentication states, keyed by challenge id.
    pub auth_states: Arc<Mutex<HashMap<Uuid, DiscoverableAuthentication>>>,
}

impl AppState {
    /// Assembles the full state from a pool and configuration.
    ///
    /// # Errors
    /// [`AuthError::Crypto`] on malformed key, origin or rp id.
    pub fn build(pool: PgPool, config: &Config) -> Result<Self, AuthError> {
        let secret_box = SecretBox::from_base64(&config.master_key_b64)?;
        let auth = Auth::new(
            Arc::new(PgUsers::new(pool.clone())),
            Arc::new(PgSessions::new(pool.clone())),
            Arc::new(PgAudit::new(pool.clone())),
            secret_box,
            config.session_ttl_days,
        );
        let origin = Url::parse(&config.webauthn_origin).map_err(|_| AuthError::Crypto)?;
        let webauthn = WebauthnBuilder::new(&config.webauthn_rp_id, &origin)
            .map_err(|_| AuthError::Crypto)?
            .rp_name("Joel")
            .build()
            .map_err(|_| AuthError::Crypto)?;
        Ok(Self {
            credentials: Arc::new(PgCredentials::new(pool.clone())),
            pool,
            auth: Arc::new(auth),
            webauthn: Arc::new(webauthn),
            reg_states: Arc::new(Mutex::new(HashMap::new())),
            auth_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}
