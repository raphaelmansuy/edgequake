//! Query-embedding cache (P-G9 / RC-14).
//!
//! WHY (First Principles): every non-Bypass query re-embeds the query string on
//! every request. For a popular query (or repeated `context_only` inspection),
//! that is a redundant, deterministic LLM/embedding-provider round-trip. The
//! embedding of a given text under a given model is a pure function of
//! (model, text), so it is safe to memoize.
//!
//! Design (DIP / DRY):
//! - `CachingEmbeddingProvider` wraps any `EmbeddingProvider` and memoizes
//!   `embed_one` results keyed by `hash(model || text)`. It delegates `embed`
//!   (batch) and all metadata methods to the inner provider unchanged, so the
//!   cache is transparent to callers and never changes semantics.
//! - `embed` (batch) is NOT cached by default: batch callers (ingestion) embed
//!   unique chunks, so caching would be pure overhead. Only the query path uses
//!   `embed_one`, which is where the repeat hit rate is high.
//! - LRU + TTL eviction (mirrors `keywords::cache::InMemoryKeywordCache`).
//! - `embedding_version` is part of the key (E27): changing the embedding model
//!   or its configuration invalidates the whole cache without a manual clear.
//! - Thread-safe via `RwLock<HashMap>`; no `async` lock held across an await
//!   on a miss (the inner provider call runs with no lock held).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use edgequake_llm::traits::EmbeddingProvider;

/// A hashed cache key bundling the embedding version (model identity) and the
/// text. Two equal texts under different models must NOT collide (E27).
fn cache_key(version: &str, text: &str) -> String {
    // Suffix the version so a model swap cannot serve a stale vector. Use a
    // delimiter unlikely to appear in either field.
    format!("{version}\u{1f}\u{1e}{text}")
}

struct Entry {
    embedding: Vec<f32>,
    expires_at: Option<Instant>,
    accessed_at: Instant,
}

/// LRU + TTL embedding cache wrapping any `EmbeddingProvider`.
///
/// Cache only `embed_one` (the query path). `embed` (batch) is delegated
/// unchanged because batch inputs (chunks) are effectively unique per call.
pub struct CachingEmbeddingProvider {
    inner: Arc<dyn EmbeddingProvider>,
    /// Bumped into the key so a model/config change invalidates everything.
    version: String,
    max_size: usize,
    ttl: Duration,
    cache: RwLock<HashMap<String, Entry>>,
    hits: RwLock<u64>,
    misses: RwLock<u64>,
}

impl CachingEmbeddingProvider {
    /// Wrap `inner` with an embedding cache of `max_size` entries and `ttl`.
    /// `version` is folded into the cache key (e.g. `"{name}/{model}/{dim}"`);
    /// change it to invalidate.
    pub fn new(inner: Arc<dyn EmbeddingProvider>, max_size: usize, ttl: Duration) -> Self {
        let version = format!("{}/{}", inner.name(), inner.model());
        Self {
            inner,
            version,
            max_size,
            ttl,
            cache: RwLock::new(HashMap::new()),
            hits: RwLock::new(0),
            misses: RwLock::new(0),
        }
    }

    /// Wrap with defaults: 10_000 entries, 1h TTL (plan-19 P-G9).
    pub fn with_defaults(inner: Arc<dyn EmbeddingProvider>) -> Self {
        Self::new(inner, 10_000, Duration::from_secs(3600))
    }

    /// Cache hit count (for tests / observability).
    pub fn hits(&self) -> u64 {
        *self.hits.read().unwrap()
    }

    /// Cache miss count.
    pub fn misses(&self) -> u64 {
        *self.misses.read().unwrap()
    }

    fn evict_if_needed(&self) {
        let mut cache = self.cache.write().unwrap();
        let now = Instant::now();
        // Drop expired entries first.
        cache.retain(|_, e| e.expires_at.map(|exp| exp > now).unwrap_or(true));
        // Then LRU-evict until under capacity.
        while cache.len() >= self.max_size {
            let oldest = cache
                .iter()
                .min_by_key(|(_, e)| e.accessed_at)
                .map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    cache.remove(&k);
                }
                None => break,
            }
        }
    }
}

#[async_trait]
impl EmbeddingProvider for CachingEmbeddingProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn max_tokens(&self) -> usize {
        self.inner.max_tokens()
    }

    fn max_batch_size(&self) -> usize {
        self.inner.max_batch_size()
    }

    // Batch embed is delegated unchanged (chunk inputs are unique per call).
    async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
        self.inner.embed(texts).await
    }

    // embed_one is the hot query path — memoize it.
    async fn embed_one(&self, text: &str) -> edgequake_llm::Result<Vec<f32>> {
        let key = cache_key(&self.version, text);
        let now = Instant::now();

        // Fast path: read lock, check for a live entry.
        {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(&key) {
                let live = entry.expires_at.map(|exp| exp > now).unwrap_or(true);
                if live {
                    *self.hits.write().unwrap() += 1;
                    return Ok(entry.embedding.clone());
                }
            }
        }

        // Miss: call the inner provider with NO cache lock held (avoid blocking
        // other readers across an await).
        *self.misses.write().unwrap() += 1;
        let embedding = self.inner.embed_one(text).await?;

        // Store. Evict first so we never exceed max_size.
        self.evict_if_needed();
        let mut cache = self.cache.write().unwrap();
        cache.insert(
            key,
            Entry {
                embedding: embedding.clone(),
                expires_at: Some(now + self.ttl),
                accessed_at: now,
            },
        );

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    #[tokio::test]
    async fn repeated_embed_one_hits_cache() {
        let mock = Arc::new(MockProvider::default());
        let cached = Arc::new(CachingEmbeddingProvider::with_defaults(
            mock as Arc<dyn EmbeddingProvider>,
        ));

        // Two identical embed_one calls: the second must be a cache hit.
        let a = cached.embed_one("hello world").await.unwrap();
        let b = cached.embed_one("hello world").await.unwrap();
        assert_eq!(a, b, "cached embedding must equal the first call");
        assert_eq!(cached.hits(), 1, "second call must hit the cache");
        assert_eq!(cached.misses(), 1, "first call must miss");
    }

    #[tokio::test]
    async fn different_texts_do_not_collide() {
        let mock = Arc::new(MockProvider::default());
        let cached = Arc::new(CachingEmbeddingProvider::with_defaults(
            mock as Arc<dyn EmbeddingProvider>,
        ));

        let _ = cached.embed_one("alpha").await.unwrap();
        let _ = cached.embed_one("beta").await.unwrap();
        // Two distinct texts → two misses, zero hits.
        assert_eq!(cached.misses(), 2);
        assert_eq!(cached.hits(), 0);
    }

    #[tokio::test]
    async fn embed_batch_is_delegated_not_cached() {
        let mock = Arc::new(MockProvider::default());
        let cached = Arc::new(CachingEmbeddingProvider::with_defaults(
            mock as Arc<dyn EmbeddingProvider>,
        ));

        // Batch embed must still work and must NOT populate the embed_one cache.
        let texts = vec!["x".to_string(), "y".to_string()];
        let out = cached.embed(&texts).await.unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(cached.hits(), 0);
        assert_eq!(
            cached.misses(),
            0,
            "batch embed must not touch the embed_one cache"
        );
    }

    #[tokio::test]
    async fn ttl_expiration_causes_re_embed() {
        let mock = Arc::new(MockProvider::default());
        let cached = Arc::new(CachingEmbeddingProvider::new(
            mock as Arc<dyn EmbeddingProvider>,
            10,
            Duration::from_millis(1),
        ));

        let _ = cached.embed_one("ephemeral").await.unwrap();
        // Wait past the TTL.
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = cached.embed_one("ephemeral").await.unwrap();
        // Both calls miss (the second because the entry expired).
        assert_eq!(cached.misses(), 2);
        assert_eq!(cached.hits(), 0);
    }
}
