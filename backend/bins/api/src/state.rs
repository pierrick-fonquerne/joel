//! Shared application state and configuration.

use std::collections::HashMap;
use std::sync::Arc;

use cipher_egide::EgideClient;
use domain::auth::crypto::SecretBox;
use domain::auth::model::AuthError;
use domain::auth::use_cases::Auth;
use persistence::auth::{PgAudit, PgCredentials, PgSessions, PgUsers};
use persistence::wealth::PgWrappedKeys;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, PasskeyRegistration, Url, Webauthn, WebauthnBuilder,
};

use crate::wealth_vault::{
    EgideUnlocker, SealedUnlocker, UNLOCK_RETRY_INTERVAL, VaultUnlocker, WealthGate,
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
    /// Egide base url (`EGIDE_URL`); the wealth vault stays sealed when absent.
    pub egide_url: Option<String>,
    /// Egide service token, read from the file named by `EGIDE_TOKEN_FILE`.
    pub egide_token: Option<String>,
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
            egide_url: std::env::var("EGIDE_URL").ok(),
            egide_token: std::env::var("EGIDE_TOKEN_FILE")
                .ok()
                .and_then(|path| read_token_file(&path)),
        })
    }
}

/// Reads the Egide token file, logging (never the token) why it is unusable.
fn read_token_file(path: &str) -> Option<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            tracing::warn!(path = %path, %error, "EGIDE_TOKEN_FILE is unreadable");
            return None;
        }
    };
    let token = content.trim();
    if token.is_empty() {
        tracing::warn!(path = %path, "EGIDE_TOKEN_FILE is empty");
        return None;
    }
    Some(token.to_owned())
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
    /// Wealth use cases behind the vault gate.
    pub wealth: Arc<WealthGate>,
}

impl AppState {
    /// Assembles the state, unlocking the wealth vault through Egide when configured.
    ///
    /// # Errors
    /// [`AuthError::Crypto`] on malformed key, origin or rp id.
    pub fn build(pool: PgPool, config: &Config) -> Result<Self, AuthError> {
        let gate = Arc::new(WealthGate::new(
            pool.clone(),
            unlocker_from(&pool, config),
            UNLOCK_RETRY_INTERVAL,
        ));
        Self::build_with_gate(pool, config, gate)
    }

    /// Assembles the state with an explicit wealth gate (tests).
    ///
    /// # Errors
    /// [`AuthError::Crypto`] on malformed key, origin or rp id.
    pub fn build_with_gate(
        pool: PgPool,
        config: &Config,
        wealth: Arc<WealthGate>,
    ) -> Result<Self, AuthError> {
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
            wealth,
        })
    }
}

fn unlocker_from(pool: &PgPool, config: &Config) -> Arc<dyn VaultUnlocker> {
    match (&config.egide_url, &config.egide_token) {
        (Some(url), Some(token)) => Arc::new(EgideUnlocker::new(
            EgideClient::new(url.clone(), token.clone()),
            PgWrappedKeys::new(pool.clone()),
        )),
        (url, token) => {
            if url.is_some() || token.is_some() {
                tracing::warn!("egide is partially configured, wealth vault stays sealed");
            }
            Arc::new(SealedUnlocker)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::read_token_file;

    #[test]
    fn trims_the_token_content() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            file.path(),
            "  secret-token 
",
        )
        .unwrap();
        assert_eq!(
            read_token_file(file.path().to_str().unwrap()).as_deref(),
            Some("secret-token")
        );
    }

    #[test]
    fn empty_file_yields_none() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            file.path(),
            " 
",
        )
        .unwrap();
        assert!(read_token_file(file.path().to_str().unwrap()).is_none());
    }

    #[test]
    fn missing_file_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("absent");
        assert!(read_token_file(path.to_str().unwrap()).is_none());
    }
}
