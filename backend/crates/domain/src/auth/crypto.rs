//! Pure cryptographic services: password hashing, TOTP, secret sealing, session tokens.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng as ArgonRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};

use super::model::AuthError;

/// Argon2id password hashing.
pub struct PasswordService;

impl PasswordService {
    /// Hashes a password into a PHC string.
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] when hashing fails.
    pub fn hash(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut ArgonRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| AuthError::Crypto)
    }

    /// Verifies a password against a stored PHC string.
    #[must_use]
    pub fn verify(password: &str, stored: &str) -> bool {
        PasswordHash::new(stored).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
    }
}

/// RFC 6238 TOTP, 6 digits, 30 second step, SHA-1 (authenticator app default).
pub struct TotpService;

impl TotpService {
    /// Generates a 20-byte random secret.
    #[must_use]
    pub fn generate_secret() -> Vec<u8> {
        let mut secret = vec![0_u8; 20];
        rand::rng().fill_bytes(&mut secret);
        secret
    }

    fn totp(secret: &[u8], account: &str) -> Result<TOTP, AuthError> {
        TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.to_vec(),
            Some("Joel".to_owned()),
            account.to_owned(),
        )
        .map_err(|_| AuthError::Crypto)
    }

    /// Computes the code for the given unix timestamp (test and seeding helper).
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] on malformed secrets.
    pub fn current_code(secret: &[u8], now_unix: u64) -> Result<String, AuthError> {
        Self::totp(secret, "joel").map(|t| t.generate(now_unix))
    }

    /// Verifies a user-provided code against the secret at the given instant.
    #[must_use]
    pub fn verify(secret: &[u8], code: &str, now_unix: u64) -> bool {
        Self::totp(secret, "joel").is_ok_and(|t| t.check(code, now_unix))
    }

    /// Builds the `otpauth://` provisioning URL for authenticator apps.
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] on malformed secrets.
    pub fn otpauth_url(secret: &[u8], account: &str) -> Result<String, AuthError> {
        Self::totp(secret, account).map(|t| t.get_url())
    }

    /// Encodes a secret in base32 for manual entry in authenticator apps.
    #[must_use]
    pub fn secret_base32(secret: &[u8]) -> String {
        Secret::Raw(secret.to_vec()).to_encoded().to_string()
    }
}

/// AES-256-GCM sealing of secrets at rest. Layout: `nonce (12) || ciphertext`.
pub struct SecretBox {
    key: [u8; 32],
}

impl SecretBox {
    /// Builds a box from a raw 32-byte key.
    #[must_use]
    pub const fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Builds a box from a base64-encoded 32-byte key (`MASTER_KEY`).
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] when the key is not 32 bytes of valid base64.
    pub fn from_base64(key_b64: &str) -> Result<Self, AuthError> {
        let bytes = STANDARD
            .decode(key_b64.trim())
            .map_err(|_| AuthError::Crypto)?;
        let key: [u8; 32] = bytes.try_into().map_err(|_| AuthError::Crypto)?;
        Ok(Self::new(key))
    }

    /// Encrypts and authenticates a payload.
    #[must_use]
    pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut out = nonce.to_vec();
        // Encryption with a fresh nonce and in-memory buffers cannot fail.
        if let Ok(ciphertext) = cipher.encrypt(&nonce, plaintext) {
            out.extend_from_slice(&ciphertext);
        }
        out
    }

    /// Decrypts a sealed payload, failing on any tampering.
    ///
    /// # Errors
    /// Returns [`AuthError::Crypto`] when the payload is malformed or forged.
    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, AuthError> {
        if sealed.len() < 13 {
            return Err(AuthError::Crypto);
        }
        let (nonce, ciphertext) = sealed.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| AuthError::Crypto)
    }
}

/// Opaque session bearer tokens: 32 random bytes, stored as SHA-256 hashes only.
pub struct SessionToken;

impl SessionToken {
    /// Generates a fresh token and its storage hash.
    #[must_use]
    pub fn generate() -> (String, [u8; 32]) {
        let mut raw = [0_u8; 32];
        rand::rng().fill_bytes(&mut raw);
        let token = URL_SAFE_NO_PAD.encode(raw);
        let hash = Self::hash(&token);
        (token, hash)
    }

    /// Hashes a presented token for storage lookup.
    #[must_use]
    pub fn hash(token: &str) -> [u8; 32] {
        Sha256::digest(token.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn password_roundtrip_verifies() {
        let hash = PasswordService::hash("correct horse battery staple").unwrap();
        assert!(PasswordService::verify(
            "correct horse battery staple",
            &hash
        ));
        assert!(!PasswordService::verify("wrong", &hash));
    }

    #[test]
    fn totp_accepts_current_code_and_rejects_garbage() {
        let secret = TotpService::generate_secret();
        let now = 1_780_000_000;
        let code = TotpService::current_code(&secret, now).unwrap();
        assert!(TotpService::verify(&secret, &code, now));
        assert!(!TotpService::verify(&secret, "000000", now));
    }

    #[test]
    fn otpauth_url_contains_issuer_and_account() {
        let secret = TotpService::generate_secret();
        let url = TotpService::otpauth_url(&secret, "pierrick@example.com").unwrap();
        assert!(url.starts_with("otpauth://totp/"));
        assert!(url.contains("Joel"));
    }

    #[test]
    fn secret_box_roundtrip_and_tamper_detection() {
        let key = [42_u8; 32];
        let sealed = SecretBox::new(key).seal(b"sensitive");
        assert_eq!(SecretBox::new(key).open(&sealed).unwrap(), b"sensitive");
        let mut tampered = sealed;
        let last = tampered.last_mut().unwrap();
        *last = last.wrapping_add(1);
        assert!(SecretBox::new(key).open(&tampered).is_err());
    }

    #[test]
    fn session_token_hash_is_stable_and_token_is_long() {
        let (token, hash) = SessionToken::generate();
        assert!(token.len() >= 43);
        assert_eq!(SessionToken::hash(&token), hash);
    }
}
