//! Classify a pasted provider secret before it is encrypted.
//!
//! The value that leaves this module is either the original API key or a small envelope. The
//! private key, refresh token, or API key is only inside that string, which the secret manager
//! encrypts. Error text names the kind that was wrong. It never repeats the secret.

use edgequake_llm::google_auth::GoogleCredentials;
use serde_json::{json, Value};

const ENVELOPE_VERSION: i64 = 1;
const MAX_SECRET_BYTES: usize = 64 * 1024;

/// What to encrypt, and the `auth_scheme` to store beside it.
#[derive(Debug)]
pub struct SealedSecret {
    pub stored: String,
    pub auth_scheme: &'static str,
}

/// A decrypted connection secret.
#[derive(Debug)]
pub enum OpenedSecret {
    /// An ordinary API key (OpenAI, Anthropic, Gemini AI Studio, …).
    ApiKey(String),
    /// A Google service-account or authorized-user JSON file.
    GoogleCredential { region: String, material: String },
    /// A Vertex Express key or an API key bound to a service account.
    GoogleApiKey(String),
}

/// Reject or wrap `raw` for `shape`. `region` is the Vertex location, not a secret.
pub fn seal_secret(shape: &str, raw: &str, region: Option<&str>) -> Result<SealedSecret, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("The secret is empty.".into());
    }
    if raw.len() > MAX_SECRET_BYTES {
        return Err("The secret is too large to store.".into());
    }
    let shape = shape.trim().to_ascii_lowercase();
    if is_vertex(&shape) {
        return seal_vertex(raw, region);
    }
    if looks_like_google_key_file(raw) {
        return Err(
            "This provider stores an API key. A Google service-account file belongs on Vertex AI."
                .into(),
        );
    }
    Ok(SealedSecret {
        stored: raw.to_string(),
        auth_scheme: if shape.contains("anthropic") {
            "x_api_key"
        } else {
            "bearer"
        },
    })
}

/// Read back a stored secret. A value from before envelopes is an API key.
/// A raw service-account file (the Test button, before save wraps it) is a Google credential.
pub fn open_secret(stored: &str) -> OpenedSecret {
    let Ok(value) = serde_json::from_str::<Value>(stored) else {
        return OpenedSecret::ApiKey(stored.to_string());
    };
    if value.get("eq_credential").and_then(Value::as_i64) != Some(ENVELOPE_VERSION) {
        let kind = value.get("type").and_then(Value::as_str);
        if matches!(kind, Some("service_account" | "authorized_user")) {
            return OpenedSecret::GoogleCredential {
                region: "us-central1".into(),
                material: stored.to_string(),
            };
        }
        return OpenedSecret::ApiKey(stored.to_string());
    }
    let material = value
        .get("material")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let region = value
        .get("region")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("us-central1")
        .to_string();
    match value.get("kind").and_then(Value::as_str) {
        Some("google_service_account" | "google_authorized_user") => {
            OpenedSecret::GoogleCredential { region, material }
        }
        Some("google_api_key") => OpenedSecret::GoogleApiKey(material),
        _ => OpenedSecret::ApiKey(stored.to_string()),
    }
}

pub fn is_vertex(shape: &str) -> bool {
    matches!(
        shape.trim().to_ascii_lowercase().as_str(),
        "vertexai" | "vertex"
    )
}

fn seal_vertex(raw: &str, region: Option<&str>) -> Result<SealedSecret, String> {
    let region = sanitize_region(region.unwrap_or("us-central1"))?;
    if raw.starts_with('{') {
        let creds: GoogleCredentials = serde_json::from_str(raw)
            .map_err(|_| "Paste the service-account JSON file (type, client_email, private_key, project_id), or an authorized-user file. The file was not one of those.".to_string())?;
        match &creds {
            GoogleCredentials::ServiceAccount {
                client_email,
                private_key,
                project_id,
                ..
            } => {
                if !client_email.contains('@') || !client_email.contains('.') {
                    return Err("The service account JSON has no client_email.".into());
                }
                if !private_key.contains("PRIVATE KEY") {
                    return Err(
                        "The service account JSON has no private_key PEM block. Download the key file again."
                            .into(),
                    );
                }
                if project_id.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(
                        "The service account JSON has no project_id. Use the key downloaded for that Google Cloud project."
                            .into(),
                    );
                }
                Ok(envelope("google_service_account", &region, raw))
            }
            GoogleCredentials::AuthorizedUser {
                client_id,
                client_secret,
                refresh_token,
                ..
            } => {
                if client_id.is_empty() || client_secret.is_empty() || refresh_token.len() < 8 {
                    return Err(
                        "The authorized-user file is missing client_id, client_secret, or refresh_token."
                            .into(),
                    );
                }
                Ok(envelope("google_authorized_user", &region, raw))
            }
        }
    } else {
        if raw.contains("PRIVATE KEY") || raw.contains("private_key") {
            return Err(
                "That looks like a service-account key, not an API key. Choose Service account JSON and paste the whole file."
                    .into(),
            );
        }
        if raw.chars().any(char::is_whitespace) || raw.len() < 20 || raw.len() > 512 {
            return Err(
                "A Vertex API key is one line, the Express key or a key bound to a service account."
                    .into(),
            );
        }
        Ok(envelope("google_api_key", &region, raw))
    }
}

fn envelope(kind: &str, region: &str, material: &str) -> SealedSecret {
    let stored = json!({
        "eq_credential": ENVELOPE_VERSION,
        "kind": kind,
        "region": region,
        "material": material,
    })
    .to_string();
    SealedSecret {
        stored,
        auth_scheme: match kind {
            "google_api_key" => "google_api_key",
            "google_authorized_user" => "google_authorized_user",
            _ => "google_service_account",
        },
    }
}

fn sanitize_region(region: &str) -> Result<String, String> {
    let region = region.trim();
    let ok = !region.is_empty()
        && region.len() <= 40
        && region
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err("Location must look like us-central1 or global.".into());
    }
    Ok(region.to_string())
}

fn looks_like_google_key_file(raw: &str) -> bool {
    raw.contains("\"private_key\"")
        || raw.contains("\"type\": \"service_account\"")
        || raw.contains("\"type\":\"service_account\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service_account() -> String {
        r#"{"type":"service_account","client_email":"vertex-express@mobioos-506015.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\nnot-a-real-key\n-----END PRIVATE KEY-----\n","project_id":"mobioos-506015","token_uri":"https://oauth2.googleapis.com/token"}"#.into()
    }

    #[test]
    fn a_service_account_file_is_sealed_without_echoing_the_key() {
        let sealed = seal_secret("vertexai", &service_account(), Some("us-central1")).unwrap();
        assert_eq!(sealed.auth_scheme, "google_service_account");
        assert!(sealed.stored.contains("mobioos-506015"));
        match open_secret(&sealed.stored) {
            OpenedSecret::GoogleCredential { region, material } => {
                assert_eq!(region, "us-central1");
                assert!(material.contains("BEGIN PRIVATE KEY"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn a_vertex_api_key_is_sealed_as_an_api_key() {
        let err = seal_secret("vertexai", "AIzaSyDaGmWKa4JsXZ-HjGw7ISLn_3namBGewQe", None).unwrap();
        assert_eq!(err.auth_scheme, "google_api_key");
        assert!(matches!(
            open_secret(&err.stored),
            OpenedSecret::GoogleApiKey(_)
        ));
    }

    #[test]
    fn openai_refuses_a_service_account_file() {
        let err = seal_secret("openai", &service_account(), None).unwrap_err();
        assert!(err.contains("Vertex"));
        assert!(!err.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn a_broken_location_is_rejected() {
        let err = seal_secret("vertex", &service_account(), Some("US Central")).unwrap_err();
        assert!(err.contains("Location"));
    }
}
