//! HTTP client with builder pattern, retry, and auth/tenant middleware.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::multipart;
use reqwest::{Client, Method, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::config::{Auth, ClientConfig, TenantContext};
use crate::error::{Error, Result};

/// Internal shared state.
struct ClientInner {
    http: Client,
    config: ClientConfig,
    auth: Auth,
    tenant: TenantContext,
}

/// The EdgeQuake SDK client. Thread-safe (`Clone + Send + Sync`).
#[derive(Clone)]
pub struct EdgeQuakeClient {
    inner: Arc<ClientInner>,
}

impl EdgeQuakeClient {
    /// Start building a new client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    // ── Resource accessors ──────────────────────────────────────────

    pub fn documents(&self) -> crate::resources::documents::DocumentsResource<'_> {
        crate::resources::documents::DocumentsResource { client: self }
    }

    pub fn graph(&self) -> crate::resources::graph::GraphResource<'_> {
        crate::resources::graph::GraphResource { client: self }
    }

    pub fn entities(&self) -> crate::resources::entities::EntitiesResource<'_> {
        crate::resources::entities::EntitiesResource { client: self }
    }

    pub fn relationships(&self) -> crate::resources::relationships::RelationshipsResource<'_> {
        crate::resources::relationships::RelationshipsResource { client: self }
    }

    pub fn query(&self) -> crate::resources::query::QueryResource<'_> {
        crate::resources::query::QueryResource { client: self }
    }

    pub fn chat(&self) -> crate::resources::chat::ChatResource<'_> {
        crate::resources::chat::ChatResource { client: self }
    }

    pub fn auth(&self) -> crate::resources::auth::AuthResource<'_> {
        crate::resources::auth::AuthResource { client: self }
    }

    pub fn users(&self) -> crate::resources::users::UsersResource<'_> {
        crate::resources::users::UsersResource { client: self }
    }

    pub fn api_keys(&self) -> crate::resources::api_keys::ApiKeysResource<'_> {
        crate::resources::api_keys::ApiKeysResource { client: self }
    }

    pub fn tenants(&self) -> crate::resources::tenants::TenantsResource<'_> {
        crate::resources::tenants::TenantsResource { client: self }
    }

    pub fn conversations(&self) -> crate::resources::conversations::ConversationsResource<'_> {
        crate::resources::conversations::ConversationsResource { client: self }
    }

    pub fn folders(&self) -> crate::resources::folders::FoldersResource<'_> {
        crate::resources::folders::FoldersResource { client: self }
    }

    pub fn tasks(&self) -> crate::resources::tasks::TasksResource<'_> {
        crate::resources::tasks::TasksResource { client: self }
    }

    pub fn pipeline(&self) -> crate::resources::pipeline::PipelineResource<'_> {
        crate::resources::pipeline::PipelineResource { client: self }
    }

    pub fn costs(&self) -> crate::resources::costs::CostsResource<'_> {
        crate::resources::costs::CostsResource { client: self }
    }

    pub fn chunks(&self) -> crate::resources::chunks::ChunksResource<'_> {
        crate::resources::chunks::ChunksResource { client: self }
    }

    pub fn provenance(&self) -> crate::resources::provenance::ProvenanceResource<'_> {
        crate::resources::provenance::ProvenanceResource { client: self }
    }

    pub fn models(&self) -> crate::resources::models::ModelsResource<'_> {
        crate::resources::models::ModelsResource { client: self }
    }

    pub fn workspaces(&self) -> crate::resources::workspaces::WorkspacesResource<'_> {
        crate::resources::workspaces::WorkspacesResource { client: self }
    }

    pub fn health(&self) -> crate::resources::health::HealthResource<'_> {
        crate::resources::health::HealthResource { client: self }
    }

    pub fn pdf(&self) -> crate::resources::pdf::PdfResource<'_> {
        crate::resources::pdf::PdfResource { client: self }
    }

    pub fn lineage(&self) -> crate::resources::lineage::LineageResource<'_> {
        crate::resources::lineage::LineageResource { client: self }
    }

    pub fn settings(&self) -> crate::resources::settings::SettingsResource<'_> {
        crate::resources::settings::SettingsResource { client: self }
    }

    // ── Low-level request helpers (used by resources) ───────────────

    /// Build a full URL from a path segment.
    pub(crate) fn url(&self, path: &str) -> Result<url::Url> {
        let base = &self.inner.config.base_url;
        let full = if path.starts_with('/') {
            format!("{}{}", base.trim_end_matches('/'), path)
        } else {
            format!("{}/{}", base.trim_end_matches('/'), path)
        };
        url::Url::parse(&full).map_err(Error::Url)
    }

    /// Execute a GET request and deserialize JSON.
    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, Option::<&()>::None).await
    }

    /// Execute a POST request with a JSON body.
    pub(crate) async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        self.request(Method::POST, path, body).await
    }

    /// Execute a PUT request with a JSON body.
    pub(crate) async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        self.request(Method::PUT, path, body).await
    }

    /// Execute a PATCH request with a JSON body.
    pub(crate) async fn patch<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        self.request(Method::PATCH, path, body).await
    }

    /// Execute a DELETE request.
    pub(crate) async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::DELETE, path, Option::<&()>::None).await
    }

    /// Execute a DELETE and discard the body (returns `()`).
    pub(crate) async fn delete_no_content(&self, path: &str) -> Result<()> {
        let resp = self.send_with_retry(Method::DELETE, path, Option::<&()>::None).await?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(Error::from_response(resp).await)
        }
    }

    /// Execute a GET request and return raw bytes (for CSV/binary downloads).
    pub(crate) async fn get_raw(&self, path: &str) -> Result<Vec<u8>> {
        let resp = self.send_with_retry(Method::GET, path, Option::<&()>::None).await?;
        let status = resp.status();
        if status.is_success() {
            resp.bytes().await.map(|b| b.to_vec()).map_err(Error::Network)
        } else {
            Err(Error::from_response(resp).await)
        }
    }

    /// Execute a POST and discard the body (returns `()`).
    pub(crate) async fn post_no_content<B: Serialize>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<()> {
        let resp = self.send_with_retry(Method::POST, path, body).await?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(Error::from_response(resp).await)
        }
    }

    /// Upload a file via multipart/form-data.
    ///
    /// WHY: PDF upload requires multipart; reqwest multipart is separate from
    /// JSON requests and needs its own send path to preserve the correct
    /// Content-Type boundary header.
    pub(crate) async fn upload_multipart<T: DeserializeOwned>(
        &self,
        path: &str,
        file_bytes: Vec<u8>,
        filename: &str,
        extra_fields: HashMap<String, String>,
    ) -> Result<T> {
        let url = self.url(path)?;

        let part = multipart::Part::bytes(file_bytes)
            .file_name(filename.to_owned())
            .mime_str("application/pdf")
            .map_err(Error::Network)?;

        let mut form = multipart::Form::new().part("file", part);
        for (key, value) in extra_fields {
            form = form.text(key, value);
        }

        let mut req = self.inner.http.post(url).multipart(form);

        // Auth headers — same as send_once
        match &self.inner.auth {
            Auth::None => {}
            Auth::ApiKey(key) => {
                req = req.header("X-API-Key", key.as_str());
            }
            Auth::Bearer(token) => {
                req = req.header(AUTHORIZATION, format!("Bearer {}", token));
            }
        }
        if let Some(tid) = &self.inner.tenant.tenant_id {
            req = req.header("X-Tenant-ID", tid.as_str());
        }
        if let Some(uid) = &self.inner.tenant.user_id {
            req = req.header("X-User-ID", uid.as_str());
        }
        if let Some(wid) = &self.inner.tenant.workspace_id {
            req = req.header("X-Workspace-ID", wid.as_str());
        }

        let resp = req.send().await.map_err(Error::Network)?;
        let status = resp.status();
        if status.is_success() {
            let bytes = resp.bytes().await.map_err(Error::Network)?;
            serde_json::from_slice(&bytes).map_err(Error::Json)
        } else {
            Err(Error::from_response(resp).await)
        }
    }

    /// Core request method with retry.
    async fn request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        let resp = self.send_with_retry(method, path, body).await?;
        let status = resp.status();
        if status.is_success() {
            let bytes = resp.bytes().await.map_err(Error::Network)?;
            serde_json::from_slice(&bytes).map_err(Error::Json)
        } else {
            Err(Error::from_response(resp).await)
        }
    }

    /// Send with retry + exponential backoff.
    async fn send_with_retry<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let max_retries = self.inner.config.max_retries;
        let backoff = self.inner.config.retry_backoff;
        let mut last_err: Option<Error> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let wait = backoff * 2u32.saturating_pow(attempt - 1);
                tokio::time::sleep(wait).await;
            }

            match self.send_once(method.clone(), path, body).await {
                Ok(resp) => {
                    // Only retry on 429 / 5xx
                    if (resp.status() == StatusCode::TOO_MANY_REQUESTS
                        || resp.status().is_server_error())
                        && attempt < max_retries {
                            last_err = Some(Error::from_response(resp).await);
                            continue;
                        }
                    return Ok(resp);
                }
                Err(e) if e.is_retryable() && attempt < max_retries => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_err.unwrap_or(Error::Config("max retries exhausted".into())))
    }

    /// Send a single request (no retry).
    async fn send_once<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let url = self.url(path)?;
        let mut req = self.inner.http.request(method, url);

        // Auth header
        match &self.inner.auth {
            Auth::None => {}
            Auth::ApiKey(key) => {
                req = req.header("X-API-Key", key.as_str());
            }
            Auth::Bearer(token) => {
                req = req.header(
                    AUTHORIZATION,
                    format!("Bearer {}", token),
                );
            }
        }

        // Tenant headers
        if let Some(tid) = &self.inner.tenant.tenant_id {
            req = req.header("X-Tenant-ID", tid.as_str());
        }
        if let Some(uid) = &self.inner.tenant.user_id {
            req = req.header("X-User-ID", uid.as_str());
        }
        if let Some(wid) = &self.inner.tenant.workspace_id {
            req = req.header("X-Workspace-ID", wid.as_str());
        }

        // Body
        if let Some(b) = body {
            req = req.json(b);
        }

        req.send().await.map_err(Error::Network)
    }

    /// Get a raw response (for streaming).
    pub(crate) async fn raw_get(&self, path: &str) -> Result<Response> {
        self.send_with_retry(Method::GET, path, Option::<&()>::None).await
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.config.base_url
    }
}

// ── Builder ────────────────────────────────────────────────────────

/// Builder for [`EdgeQuakeClient`].
pub struct ClientBuilder {
    config: ClientConfig,
    auth: Auth,
    tenant: TenantContext,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            config: ClientConfig::default(),
            auth: Auth::None,
            tenant: TenantContext::default(),
        }
    }
}

impl ClientBuilder {
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.auth = Auth::ApiKey(key.into());
        self
    }

    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = Auth::Bearer(token.into());
        self
    }

    pub fn tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant.tenant_id = Some(id.into());
        self
    }

    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.tenant.user_id = Some(id.into());
        self
    }

    pub fn workspace_id(mut self, id: impl Into<String>) -> Self {
        self.tenant.workspace_id = Some(id.into());
        self
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.config.timeout = d;
        self
    }

    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.config.connect_timeout = d;
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.config.max_retries = n;
        self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.config.user_agent = ua.into();
        self
    }

    /// Build the client. Fails if the base URL is invalid.
    pub fn build(self) -> Result<EdgeQuakeClient> {
        // Validate base URL
        let _ = url::Url::parse(&self.config.base_url)
            .map_err(|_| Error::Config(format!("invalid base_url: {}", self.config.base_url)))?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&self.config.user_agent)
                .unwrap_or_else(|_| HeaderValue::from_static("edgequake-rust-sdk/0.1.0")),
        );

        let http = Client::builder()
            .timeout(self.config.timeout)
            .connect_timeout(self.config.connect_timeout)
            .default_headers(headers)
            .build()
            .map_err(Error::Network)?;

        Ok(EdgeQuakeClient {
            inner: Arc::new(ClientInner {
                http,
                config: self.config,
                auth: self.auth,
                tenant: self.tenant,
            }),
        })
    }
}

impl std::fmt::Debug for EdgeQuakeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeQuakeClient")
            .field("base_url", &self.inner.config.base_url)
            .finish()
    }
}
