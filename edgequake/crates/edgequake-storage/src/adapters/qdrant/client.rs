use std::time::Duration;

use edgequake_storage_contracts::{AccessError, AccessResult};
use reqwest::{header, Method};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use super::errors::{http_error, transport_error};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Typed HTTP client for a single immutable Qdrant data binding.
#[derive(Clone, Debug)]
pub struct QdrantClient {
    http: reqwest::Client,
    base_url: String,
    binding_id: Uuid,
    collection_name: String,
}

impl QdrantClient {
    pub fn new(base_url: impl AsRef<str>, binding_id: Uuid) -> AccessResult<Self> {
        Self::with_options(base_url, binding_id, DEFAULT_TIMEOUT, None)
    }

    pub fn with_options(
        base_url: impl AsRef<str>,
        binding_id: Uuid,
        timeout: Duration,
        api_key: Option<&str>,
    ) -> AccessResult<Self> {
        let base_url = base_url.as_ref().trim().trim_end_matches('/');
        let parsed = reqwest::Url::parse(base_url)
            .map_err(|error| AccessError::InvalidInput(format!("invalid Qdrant URL: {error}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(AccessError::InvalidInput(
                "Qdrant URL must use http or https".into(),
            ));
        }
        if timeout.is_zero() {
            return Err(AccessError::InvalidInput(
                "Qdrant timeout must be greater than zero".into(),
            ));
        }

        let mut headers = header::HeaderMap::new();
        if let Some(api_key) = api_key.filter(|value| !value.trim().is_empty()) {
            let value = header::HeaderValue::from_str(api_key).map_err(|error| {
                AccessError::InvalidInput(format!("invalid Qdrant API key header: {error}"))
            })?;
            headers.insert("api-key", value);
        }
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .build()
            .map_err(transport_error)?;

        Ok(Self {
            http,
            base_url: base_url.to_string(),
            binding_id,
            collection_name: collection_name(binding_id),
        })
    }

    pub fn from_env_url(url: impl AsRef<str>, binding_id: Uuid) -> AccessResult<Self> {
        let api_key = std::env::var("QDRANT_API_KEY").ok();
        Self::with_options(url, binding_id, DEFAULT_TIMEOUT, api_key.as_deref())
    }

    pub const fn binding_id(&self) -> Uuid {
        self.binding_id
    }

    pub fn collection_name(&self) -> &str {
        &self.collection_name
    }

    pub async fn health(&self) -> AccessResult<()> {
        self.request_bytes::<()>(Method::GET, "/readyz", None, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_json<T: DeserializeOwned>(&self, path: &str) -> AccessResult<T> {
        let bytes = self
            .request_bytes::<()>(Method::GET, path, None, None)
            .await?;
        serde_json::from_slice(&bytes)
            .map_err(|error| AccessError::CorruptData(format!("invalid Qdrant JSON: {error}")))
    }

    pub(crate) async fn put_json<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> AccessResult<Vec<u8>> {
        self.request_bytes(Method::PUT, path, Some(body), None)
            .await
    }

    pub(crate) async fn post_json<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        timeout: Option<Duration>,
    ) -> AccessResult<Vec<u8>> {
        self.request_bytes(Method::POST, path, Some(body), timeout)
            .await
    }

    pub(crate) async fn delete(&self, path: &str) -> AccessResult<Vec<u8>> {
        self.request_bytes::<()>(Method::DELETE, path, None, None)
            .await
    }

    async fn request_bytes<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        timeout: Option<Duration>,
    ) -> AccessResult<Vec<u8>> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.http.request(method, url);
        if let Some(body) = body {
            request = request.json(body);
        }
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(transport_error)?;
        if !status.is_success() {
            return Err(http_error(status, &String::from_utf8_lossy(&bytes)));
        }
        Ok(bytes.to_vec())
    }
}

/// Collection names contain no tenant or user input.
pub fn collection_name(binding_id: Uuid) -> String {
    format!("eq_{}", binding_id.simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_name_depends_only_on_binding_uuid() {
        let binding_id = Uuid::parse_str("67b9d12e-51f0-4d95-909c-04305e425e0d").unwrap();
        assert_eq!(
            collection_name(binding_id),
            "eq_67b9d12e51f04d95909c04305e425e0d"
        );
    }

    #[test]
    fn invalid_url_is_rejected_before_network_io() {
        let error = QdrantClient::new("file:///tmp/qdrant", Uuid::nil()).unwrap_err();
        assert!(matches!(error, AccessError::InvalidInput(_)));
    }
}
