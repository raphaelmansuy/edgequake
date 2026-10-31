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
