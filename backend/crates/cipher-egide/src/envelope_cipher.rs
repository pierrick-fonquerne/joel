//! Envelope encryption: a 256-bit data key, wrapped by Egide, encrypts every
//! field locally with AES-256-GCM. The clear key lives only in memory.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use domain::wealth::{CipherContext, FieldCipher, WealthError, WrappedKeyStore};
use zeroize::Zeroizing;

use crate::egide_client::{EgideClient, EgideError};

/// Name of the Egide Transit key that wraps the wealth data key.
pub const WEALTH_KEY_NAME: &str = "joel-wealth";

/// Why the vault could not be unlocked. `Display` never contains key material.
#[derive(Debug, thiserror::Error)]
pub enum UnlockError {
    /// Egide refused or could not answer.
    #[error("egide: {0}")]
    Egide(#[from] EgideError),
    /// The wrapped key store failed.
    #[error("wrapped key store: {0}")]
    Store(#[from] WealthError),
    /// Rewrap requested while no key was ever stored.
    #[error("no wrapped key is stored")]
    NoStoredKey,
    /// Egide returned a key that is not 32 bytes long.
    #[error("data key has an invalid length")]
    InvalidKeyLength,
}

/// [`FieldCipher`] backed by an Egide-wrapped data key.
pub struct EgideEnvelopeCipher {
    key: Zeroizing<[u8; 32]>,
}

impl EgideEnvelopeCipher {
    /// Builds a cipher from a clear data key (tests and unlock).
    #[must_use]
    pub const fn from_key(key: Zeroizing<[u8; 32]>) -> Self {
        Self { key }
    }

    /// Unwraps the stored data key, or generates and stores one on first start.
    ///
    /// # Errors
    /// See [`UnlockError`].
    pub async fn unlock(
        client: &EgideClient,
        store: &dyn WrappedKeyStore,
    ) -> Result<Self, UnlockError> {
        let clear = if let Some(wrapped) = store.current().await? {
            client.decrypt(WEALTH_KEY_NAME, &wrapped).await?
        } else {
            let datakey = client.generate_datakey(WEALTH_KEY_NAME).await?;
            to_key(&datakey.plaintext)?;
            store.insert(&datakey.ciphertext, WEALTH_KEY_NAME).await?;
            datakey.plaintext
        };
        Ok(Self::from_key(to_key(&clear)?))
    }
}

fn to_key(bytes: &[u8]) -> Result<Zeroizing<[u8; 32]>, UnlockError> {
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| UnlockError::InvalidKeyLength)?;
    Ok(Zeroizing::new(array))
}

/// Re-wraps the current data key with the latest Egide key version and stores
/// it as a new version. Encrypted fields are untouched.
///
/// # Errors
/// See [`UnlockError`].
pub async fn rewrap_stored_key(
    client: &EgideClient,
    store: &dyn WrappedKeyStore,
) -> Result<(), UnlockError> {
    let current = store.current().await?.ok_or(UnlockError::NoStoredKey)?;
    let rewrapped = client.rewrap(WEALTH_KEY_NAME, &current).await?;
    if rewrapped != current {
        store.insert(&rewrapped, WEALTH_KEY_NAME).await?;
    }
    Ok(())
}

impl FieldCipher for EgideEnvelopeCipher {
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.as_slice()));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let aad = context.aad();
        let sealed = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| WealthError::Cipher)?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&sealed);
        Ok(out)
    }

    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, WealthError> {
        let (nonce, sealed) = ciphertext.split_at_checked(12).ok_or(WealthError::Cipher)?;
        if sealed.is_empty() {
            return Err(WealthError::Cipher);
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.as_slice()));
        let aad = context.aad();
        cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: sealed,
                    aad: &aad,
                },
            )
            .map_err(|_| WealthError::Cipher)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use domain::wealth::test_support::FakeWrappedKeys;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

    fn context(column: &'static str, row_id: Uuid) -> CipherContext {
        CipherContext {
            table: "wealth_valuations",
            column,
            row_id,
        }
    }

    fn cipher() -> EgideEnvelopeCipher {
        EgideEnvelopeCipher::from_key(Zeroizing::new([7_u8; 32]))
    }

    #[test]
    fn roundtrip_restores_the_plaintext() {
        let ctx = context("amount", Uuid::new_v4());
        let sealed = cipher().encrypt(b"12345.67", &ctx).unwrap();
        assert_ne!(&sealed[12..], b"12345.67");
        assert_eq!(cipher().decrypt(&sealed, &ctx).unwrap(), b"12345.67");
    }

    #[test]
    fn decrypt_fails_with_another_row_or_column() {
        let row_id = Uuid::new_v4();
        let sealed = cipher()
            .encrypt(b"12345.67", &context("amount", row_id))
            .unwrap();
        assert_eq!(
            cipher().decrypt(&sealed, &context("amount", Uuid::new_v4())),
            Err(WealthError::Cipher)
        );
        assert_eq!(
            cipher().decrypt(&sealed, &context("notes", row_id)),
            Err(WealthError::Cipher)
        );
    }

    #[test]
    fn decrypt_fails_on_tampering_and_truncation() {
        let ctx = context("amount", Uuid::new_v4());
        let mut sealed = cipher().encrypt(b"12345.67", &ctx).unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert_eq!(cipher().decrypt(&sealed, &ctx), Err(WealthError::Cipher));
        assert_eq!(cipher().decrypt(&[1, 2, 3], &ctx), Err(WealthError::Cipher));
    }

    #[test]
    fn two_encryptions_differ_thanks_to_random_nonces() {
        let ctx = context("amount", Uuid::new_v4());
        assert_ne!(
            cipher().encrypt(b"1", &ctx).unwrap(),
            cipher().encrypt(b"1", &ctx).unwrap()
        );
    }

    #[tokio::test]
    async fn first_unlock_generates_and_stores_the_wrapped_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/datakey/joel-wealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "plaintext": KEY_B64, "ciphertext": "egide:v1:wrapped" }),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeWrappedKeys::default();

        EgideEnvelopeCipher::unlock(&EgideClient::new(server.uri(), "t"), &store)
            .await
            .unwrap();

        assert_eq!(
            store.current().await.unwrap().as_deref(),
            Some("egide:v1:wrapped")
        );
    }

    #[tokio::test]
    async fn later_unlock_decrypts_the_stored_key_and_reads_previous_data() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/decrypt/joel-wealth"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "plaintext": KEY_B64 })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeWrappedKeys::default();
        store
            .insert("egide:v1:wrapped", WEALTH_KEY_NAME)
            .await
            .unwrap();
        let mut key = [0_u8; 32];
        for (index, byte) in key.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        let ctx = context("amount", Uuid::new_v4());
        let sealed = EgideEnvelopeCipher::from_key(Zeroizing::new(key))
            .encrypt(b"42", &ctx)
            .unwrap();

        let unlocked = EgideEnvelopeCipher::unlock(&EgideClient::new(server.uri(), "t"), &store)
            .await
            .unwrap();

        assert_eq!(unlocked.decrypt(&sealed, &ctx).unwrap(), b"42");
    }

    #[tokio::test]
    async fn sealed_egide_and_short_keys_fail_to_unlock() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let result = EgideEnvelopeCipher::unlock(
            &EgideClient::new(server.uri(), "t"),
            &FakeWrappedKeys::default(),
        )
        .await;
        assert!(matches!(
            result,
            Err(UnlockError::Egide(EgideError::Sealed))
        ));

        let short = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "plaintext": "AAEC", "ciphertext": "egide:v1:short" }),
            ))
            .mount(&short)
            .await;
        let result = EgideEnvelopeCipher::unlock(
            &EgideClient::new(short.uri(), "t"),
            &FakeWrappedKeys::default(),
        )
        .await;
        assert!(matches!(result, Err(UnlockError::InvalidKeyLength)));
    }

    #[tokio::test]
    async fn rewrap_appends_a_new_key_version() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/rewrap/joel-wealth"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ciphertext": "egide:v2:wrapped" })),
            )
            .mount(&server)
            .await;
        let store = FakeWrappedKeys::default();
        store
            .insert("egide:v1:wrapped", WEALTH_KEY_NAME)
            .await
            .unwrap();

        rewrap_stored_key(&EgideClient::new(server.uri(), "t"), &store)
            .await
            .unwrap();

        assert_eq!(
            store.current().await.unwrap().as_deref(),
            Some("egide:v2:wrapped")
        );
        assert!(matches!(
            rewrap_stored_key(
                &EgideClient::new(server.uri(), "t"),
                &FakeWrappedKeys::default()
            )
            .await,
            Err(UnlockError::NoStoredKey)
        ));
    }
}
