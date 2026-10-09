//! SPEC-163 envelope encryption and redacted secret strings.
//!
//! Keys are AES-256-GCM. The process key is `EDGEQUAKE_SECRETS_KEY`
//! (32 raw bytes, or base64, or 64-char hex). `key_id` defaults to `v1`.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const DEFAULT_KEY_ID: &str = "v1";
pub const ENV_SECRETS_KEY: &str = "EDGEQUAKE_SECRETS_KEY";
pub const ENV_SECRETS_KEY_ID: &str = "EDGEQUAKE_SECRETS_KEY_ID";

#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("EDGEQUAKE_SECRETS_KEY is not set")]
    MissingKey,
    #[error("EDGEQUAKE_SECRETS_KEY must be 32 bytes (raw, base64, or 64-char hex)")]
    InvalidKey,
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed (wrong key or tampered ciphertext)")]
    Decrypt,
}

/// Redacted secret. `Display`/`Debug` never print the value.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn fingerprint(&self) -> String {
        fingerprint(self.0.as_bytes())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// AES-256-GCM envelope produced by [`encrypt`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,
}

pub fn fingerprint(bytes: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    let n = h.finish();
    format!("eqk_{:016x}", n)
}

pub fn parse_key_material(raw: &str) -> Result<[u8; 32], SecretsError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SecretsError::InvalidKey);
    }
    if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let decoded = hex::decode(trimmed).map_err(|_| SecretsError::InvalidKey)?;
        return decoded.try_into().map_err(|_| SecretsError::InvalidKey);
    }
    if let Ok(decoded) = B64.decode(trimmed) {
        if decoded.len() == 32 {
            return decoded.try_into().map_err(|_| SecretsError::InvalidKey);
        }
    }
    if trimmed.len() == 32 {
        let mut key = [0u8; 32];
        key.copy_from_slice(trimmed.as_bytes());
        return Ok(key);
    }
    Err(SecretsError::InvalidKey)
}

pub fn key_from_env() -> Result<([u8; 32], String), SecretsError> {
    let raw = std::env::var(ENV_SECRETS_KEY).map_err(|_| SecretsError::MissingKey)?;
    let key = parse_key_material(&raw)?;
    let key_id = std::env::var(ENV_SECRETS_KEY_ID).unwrap_or_else(|_| DEFAULT_KEY_ID.to_string());
    Ok((key, key_id))
}

pub fn secrets_configured() -> bool {
    key_from_env().is_ok()
}

pub fn encrypt(plaintext: &[u8], key: &[u8; 32], key_id: &str) -> Result<Envelope, SecretsError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| SecretsError::InvalidKey)?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| SecretsError::Encrypt)?;
    Ok(Envelope {
        ciphertext,
        nonce: nonce_bytes.to_vec(),
        key_id: key_id.to_string(),
    })
}

pub fn decrypt(envelope: &Envelope, key: &[u8; 32]) -> Result<Vec<u8>, SecretsError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| SecretsError::InvalidKey)?;
    if envelope.nonce.len() != 12 {
        return Err(SecretsError::Decrypt);
    }
    let nonce = Nonce::from_slice(&envelope.nonce);
    cipher
        .decrypt(nonce, envelope.ciphertext.as_ref())
        .map_err(|_| SecretsError::Decrypt)
}

pub fn encrypt_string(value: &str) -> Result<Envelope, SecretsError> {
    let (key, key_id) = key_from_env()?;
    encrypt(value.as_bytes(), &key, &key_id)
}

pub fn decrypt_string(envelope: &Envelope) -> Result<SecretString, SecretsError> {
    let (key, _) = key_from_env()?;
    let bytes = decrypt(envelope, &key)?;
    let s = String::from_utf8(bytes).map_err(|_| SecretsError::Decrypt)?;
    Ok(SecretString::new(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_redacts_debug() {
        let s = SecretString::new("sk-live-secret");
        assert!(!format!("{s:?}").contains("sk-live"));
        assert_eq!(format!("{s}"), "***");
        assert_eq!(s.expose(), "sk-live-secret");
    }

    #[test]
    fn round_trip_and_tamper() {
        let key = [7u8; 32];
        let env = encrypt(b"hello-world", &key, "v1").unwrap();
        assert_eq!(decrypt(&env, &key).unwrap(), b"hello-world");
        let mut bad = env.clone();
        bad.ciphertext[0] ^= 0xff;
        assert!(decrypt(&bad, &key).is_err());
        let wrong = [8u8; 32];
        assert!(decrypt(&env, &wrong).is_err());
    }

    #[test]
    fn parse_hex_and_b64() {
        let hex_key = "aa".repeat(32);
        assert!(parse_key_material(&hex_key).is_ok());
        let raw = [9u8; 32];
        let b64 = B64.encode(raw);
        assert_eq!(parse_key_material(&b64).unwrap(), raw);
    }

    #[test]
    fn missing_env_is_fail_closed() {
        std::env::remove_var(ENV_SECRETS_KEY);
        assert!(matches!(key_from_env(), Err(SecretsError::MissingKey)));
        assert!(!secrets_configured());
    }
}
