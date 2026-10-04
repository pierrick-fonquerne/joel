//! Minimal HTTP client for the Egide Transit engine.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Egide failures. `Display` never contains a token, a key or a ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EgideError {
    /// Egide answered 503: it is sealed.
    #[error("egide is sealed")]
    Sealed,
    /// Egide refused the token (401 or 403).
    #[error("egide refused the token")]
    Unauthorized,
    /// Any other non-success status.
    #[error("egide answered status {0}")]
    Unexpected(u16),
    /// Network failure before any answer.
    #[error("egide unreachable: {0}")]
    Transport(String),
    /// The answer could not be decoded.
    #[error("egide answer is malformed")]
    Malformed,
}

/// A freshly generated data key: the plaintext to use now, the ciphertext to store.
pub struct GeneratedDatakey {
    /// Raw key bytes, wiped on drop.
    pub plaintext: Zeroizing<Vec<u8>>,
    /// Key wrapped by Egide (`egide:vN:...`).
    pub ciphertext: String,
}

/// Egide Transit client authenticated by a service token.
pub struct EgideClient {
    http: reqwest::Client,
    base_url: String,
    token: Zeroizing<String>,
}

#[derive(Serialize)]
struct CiphertextBody<'a> {
    ciphertext: &'a str,
}

#[derive(Deserialize)]
struct PlaintextAnswer {
    plaintext: String,
}

#[derive(Deserialize)]
struct CiphertextAnswer {
    ciphertext: String,
}

#[derive(Deserialize)]
struct DatakeyAnswer {
    plaintext: String,
    ciphertext: String,
}

impl EgideClient {
    /// Builds a client for `base_url` (for example `http://egide:8200`).
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: Zeroizing::new(token.into()),
        }
    }

    /// Generates a data key wrapped under `key_name`.
    ///
    /// # Errors
    /// See [`EgideError`].
    pub async fn generate_datakey(&self, key_name: &str) -> Result<GeneratedDatakey, EgideError> {
        let answer: DatakeyAnswer = self.post(&format!("datakey/{key_name}"), None).await?;
        Ok(GeneratedDatakey {
            plaintext: decode_key(&answer.plaintext)?,
            ciphertext: answer.ciphertext,
        })
    }

    /// Unwraps a data key.
    ///
    /// # Errors
    /// See [`EgideError`].
    pub async fn decrypt(
        &self,
        key_name: &str,
        ciphertext: &str,
    ) -> Result<Zeroizing<Vec<u8>>, EgideError> {
        let answer: PlaintextAnswer = self
            .post(
                &format!("decrypt/{key_name}"),
                Some(&CiphertextBody { ciphertext }),
            )
            .await?;
        decode_key(&answer.plaintext)
    }

    /// Re-wraps a ciphertext with the latest version of `key_name`.
    ///
    /// # Errors
    /// See [`EgideError`].
    pub async fn rewrap(&self, key_name: &str, ciphertext: &str) -> Result<String, EgideError> {
        let answer: CiphertextAnswer = self
            .post(
                &format!("rewrap/{key_name}"),
                Some(&CiphertextBody { ciphertext }),
            )
            .await?;
        Ok(answer.ciphertext)
    }

    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        operation: &str,
        body: Option<&CiphertextBody<'_>>,
    ) -> Result<T, EgideError> {
        let mut request = self
            .http
            .post(format!("{}/v1/transit/{operation}", self.base_url))
            .bearer_auth(self.token.as_str());
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|error| EgideError::Transport(error.without_url().to_string()))?;
        match response.status() {
            status if status.is_success() => response
                .json::<T>()
                .await
                .map_err(|_| EgideError::Malformed),
            StatusCode::SERVICE_UNAVAILABLE => Err(EgideError::Sealed),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(EgideError::Unauthorized),
            status => Err(EgideError::Unexpected(status.as_u16())),
        }
    }
}

fn decode_key(encoded: &str) -> Result<Zeroizing<Vec<u8>>, EgideError> {
    STANDARD
        .decode(encoded)
        .map(Zeroizing::new)
        .map_err(|_| EgideError::Malformed)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

    #[tokio::test]
    async fn generate_datakey_sends_bearer_and_decodes_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/datakey/joel-wealth"))
            .and(header("authorization", "Bearer egst_test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "plaintext": KEY_B64, "ciphertext": "egide:v1:wrapped" }),
            ))
            .mount(&server)
            .await;
        let client = EgideClient::new(server.uri(), "egst_test");

        let datakey = client.generate_datakey("joel-wealth").await.unwrap();

        assert_eq!(datakey.plaintext.len(), 32);
        assert_eq!(datakey.plaintext[1], 1);
        assert_eq!(datakey.ciphertext, "egide:v1:wrapped");
    }

    #[tokio::test]
    async fn decrypt_and_rewrap_send_the_ciphertext() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/decrypt/joel-wealth"))
            .and(body_json(
                serde_json::json!({ "ciphertext": "egide:v1:wrapped" }),
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "plaintext": KEY_B64 })),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/rewrap/joel-wealth"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ciphertext": "egide:v2:wrapped" })),
            )
            .mount(&server)
            .await;
        let client = EgideClient::new(server.uri(), "egst_test");

        assert_eq!(
            client
                .decrypt("joel-wealth", "egide:v1:wrapped")
                .await
                .unwrap()
                .len(),
            32
        );
        assert_eq!(
            client
                .rewrap("joel-wealth", "egide:v1:wrapped")
                .await
                .unwrap(),
            "egide:v2:wrapped"
        );
    }

    #[tokio::test]
    async fn maps_statuses_to_errors() {
        for (status, expected) in [
            (503, EgideError::Sealed),
            (401, EgideError::Unauthorized),
            (403, EgideError::Unauthorized),
            (500, EgideError::Unexpected(500)),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(status))
                .mount(&server)
                .await;
            let client = EgideClient::new(server.uri(), "egst_test");
            assert_eq!(
                client.generate_datakey("joel-wealth").await.err(),
                Some(expected)
            );
        }
    }

    #[tokio::test]
    async fn malformed_answer_and_unreachable_server_are_reported() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "plaintext": "%%%" })),
            )
            .mount(&server)
            .await;
        let client = EgideClient::new(server.uri(), "egst_test");
        assert_eq!(
            client.decrypt("joel-wealth", "x").await.err(),
            Some(EgideError::Malformed)
        );

        let unreachable = EgideClient::new("http://127.0.0.1:9", "egst_test");
        assert!(matches!(
            unreachable.decrypt("joel-wealth", "x").await,
            Err(EgideError::Transport(_))
        ));
    }
}
