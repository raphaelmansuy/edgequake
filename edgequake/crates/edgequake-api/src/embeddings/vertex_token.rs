//! SPEC-180 08 Auth: an access token for Vertex, cached and refreshed.
//!
//! `GOOGLE_ACCESS_TOKEN` wins. Otherwise `gcloud auth print-access-token` for the active
//! account. A token is reused until five minutes before its assumed expiry (gcloud tokens
//! live one hour), and is minted again after a 401. The token is never logged.

use std::sync::Arc;
use std::time::{Duration, Instant};

use edgequake_core::multimodal_embed::{embed_codes, EmbedError};
use tokio::sync::Mutex;

/// gcloud and metadata tokens last one hour.
const ASSUMED_LIFETIME: Duration = Duration::from_secs(3600);
/// Mint a new token once less than this remains.
const REFRESH_MARGIN: Duration = Duration::from_secs(300);

/// Where a token comes from. A seam so tests never run `gcloud`.
#[async_trait::async_trait]
pub trait TokenSource: Send + Sync {
    async fn mint(&self) -> Result<String, EmbedError>;
}

/// `GOOGLE_ACCESS_TOKEN`, else `gcloud auth print-access-token`.
pub struct EnvOrGcloudSource;

#[async_trait::async_trait]
impl TokenSource for EnvOrGcloudSource {
    async fn mint(&self) -> Result<String, EmbedError> {
        /*
         * SPEC-185 — prefer the identity verified against the configured project.
         *
         * This used to always take the gcloud account, while chat preferred the ADC file. When
         * those are different principals, one path worked and the other failed 403 on the same
         * project. Asking the shared verifier first is what makes chat and embedding agree; the
         * gcloud path stays as the fallback for the case where no project is configured.
         */
        if let Some(project) = crate::providers::vertex_identity::vertex_project() {
            let model = std::env::var("EDGEQUAKE_VERTEX_EMBEDDING_MODEL")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "gemini-embedding-2".to_string());
            match crate::providers::vertex_identity::select_identity(&project, &model).await {
                Ok(access) => return Ok(access.token),
                // Verified identity unavailable: fall through to the legacy ladder so a deployment
                // that never configured a project keeps working exactly as before.
                Err(err) => {
                    tracing::debug!(
                        project = %project,
                        error = %err,
                        "Vertex identity verification failed; using the gcloud token"
                    );
                }
            }
        }
        if let Ok(token) = std::env::var("GOOGLE_ACCESS_TOKEN") {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }
        let output = tokio::process::Command::new("gcloud")
            .args(["auth", "print-access-token"])
            .output()
            .await
            .map_err(|e| auth_error(format!("cannot run gcloud: {e}")))?;
        if !output.status.success() {
            return Err(auth_error(
                "gcloud has no active account; set GOOGLE_ACCESS_TOKEN or run gcloud auth login",
            ));
        }
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            return Err(auth_error("gcloud returned an empty token"));
        }
        Ok(token)
    }
}

fn auth_error(message: impl Into<String>) -> EmbedError {
    EmbedError::new(embed_codes::AUTH, message)
}

/// Mints a Google access token from a stored service-account JSON.
///
/// The JSON stays in memory. It is parsed on each refresh and never logged.
pub struct StoredServiceAccount {
    material: String,
}

impl StoredServiceAccount {
    pub fn new(material: impl Into<String>) -> Self {
        Self {
            material: material.into(),
        }
    }
}

#[async_trait::async_trait]
impl TokenSource for StoredServiceAccount {
    async fn mint(&self) -> Result<String, EmbedError> {
        let creds: edgequake_llm::google_auth::GoogleCredentials =
            serde_json::from_str(&self.material).map_err(|_| {
                auth_error("The stored Google credential is not a service-account file.")
            })?;
        let token = creds.into_source().refresh().await.map_err(|_| {
            auth_error("Could not mint a Google access token. The key may have been revoked.")
        })?;
        Ok(token.token)
    }
}

struct Cached {
    token: String,
    minted_at: Instant,
}

/// Caches one token. Clones share the cache, so one process mints once per hour.
#[derive(Clone)]
pub struct VertexTokenProvider {
    source: Arc<dyn TokenSource>,
    cached: Arc<Mutex<Option<Cached>>>,
}

impl VertexTokenProvider {
    pub fn new(source: Arc<dyn TokenSource>) -> Self {
        Self {
            source,
            cached: Arc::new(Mutex::new(None)),
        }
    }

    /// The process-wide provider over [`EnvOrGcloudSource`].
    pub fn from_env() -> Self {
        static SHARED: std::sync::OnceLock<VertexTokenProvider> = std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| Self::new(Arc::new(EnvOrGcloudSource)))
            .clone()
    }

    /// The cached token, or a fresh one when it is missing or near expiry.
    pub async fn token(&self) -> Result<String, EmbedError> {
        let mut guard = self.cached.lock().await;
        if let Some(cached) = guard.as_ref() {
            if cached.minted_at.elapsed() + REFRESH_MARGIN < ASSUMED_LIFETIME {
                return Ok(cached.token.clone());
            }
        }
        let token = self.source.mint().await?;
        *guard = Some(Cached {
            token: token.clone(),
            minted_at: Instant::now(),
        });
        Ok(token)
    }

    /// Drop the cache and mint again (after a 401).
    pub async fn refresh(&self) -> Result<String, EmbedError> {
        *self.cached.lock().await = None;
        self.token().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Counting(AtomicUsize);

    #[async_trait::async_trait]
    impl TokenSource for Counting {
        async fn mint(&self) -> Result<String, EmbedError> {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            Ok(format!("token-{n}"))
        }
    }

    #[tokio::test]
    async fn token_is_cached_until_refreshed() {
        let provider = VertexTokenProvider::new(Arc::new(Counting(AtomicUsize::new(0))));
        assert_eq!(provider.token().await.unwrap(), "token-0");
        assert_eq!(provider.token().await.unwrap(), "token-0");
        assert_eq!(provider.refresh().await.unwrap(), "token-1");
        assert_eq!(provider.token().await.unwrap(), "token-1");
    }
}
