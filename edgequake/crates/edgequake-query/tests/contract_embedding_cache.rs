//! Contract test for P-G9 (RC-14): query embedding cache.
//!
//! Verifies that wrapping an `EmbeddingProvider` in
//! `CachingEmbeddingProvider` skips redundant `embed_one` round-trips for
//! identical query text, while `embed` (batch, ingestion) is delegated
//! unchanged and never reads the cache.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::traits::EmbeddingProvider;
use edgequake_llm::MockProvider;
use edgequake_query::cache::CachingEmbeddingProvider;

/// Wraps an embedding provider and counts `embed_one` calls so the contract
/// test can assert the cache short-circuits the inner provider.
struct CountingEmbedding {
    inner: Arc<dyn EmbeddingProvider>,
    embed_one_calls: Arc<AtomicUsize>,
    embed_calls: Arc<AtomicUsize>,
}

#[async_trait]
impl EmbeddingProvider for CountingEmbedding {
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
    async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
        self.embed_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.embed(texts).await
    }
    async fn embed_one(&self, text: &str) -> edgequake_llm::Result<Vec<f32>> {
        self.embed_one_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.embed_one(text).await
    }
}

#[tokio::test]
async fn repeated_query_embeddings_skip_inner_provider() {
    let mock: Arc<dyn EmbeddingProvider> = Arc::new(MockProvider::default());
    let embed_one_calls = Arc::new(AtomicUsize::new(0));
    let embed_calls = Arc::new(AtomicUsize::new(0));
    let counting = Arc::new(CountingEmbedding {
        inner: mock,
        embed_one_calls: embed_one_calls.clone(),
        embed_calls: embed_calls.clone(),
    }) as Arc<dyn EmbeddingProvider>;

    let cached = Arc::new(CachingEmbeddingProvider::with_defaults(counting));

    // Three identical query embeddings.
    for _ in 0..3 {
        let _ = cached.embed_one("what is GraphRAG?").await.unwrap();
    }

    // The inner provider's embed_one must be called exactly once; the other
    // two calls must be served from the cache.
    assert_eq!(
        embed_one_calls.load(Ordering::SeqCst),
        1,
        "repeated identical queries must hit the cache, not the inner provider"
    );
    assert_eq!(cached.hits(), 2, "two cache hits expected");
    assert_eq!(cached.misses(), 1, "one cache miss expected (first call)");
}

#[tokio::test]
async fn batch_embed_bypasses_cache_and_calls_inner() {
    let mock: Arc<dyn EmbeddingProvider> = Arc::new(MockProvider::default());
    let embed_one_calls = Arc::new(AtomicUsize::new(0));
    let embed_calls = Arc::new(AtomicUsize::new(0));
    let counting = Arc::new(CountingEmbedding {
        inner: mock,
        embed_one_calls: embed_one_calls.clone(),
        embed_calls: embed_calls.clone(),
    }) as Arc<dyn EmbeddingProvider>;

    let cached = Arc::new(CachingEmbeddingProvider::with_defaults(counting));

    let texts = vec!["chunk a".to_string(), "chunk b".to_string()];
    let out = cached.embed(&texts).await.unwrap();
    assert_eq!(out.len(), 2);

    // Batch embed must reach the inner provider and must NOT touch the
    // embed_one cache (ingestion inputs are unique per call).
    assert_eq!(
        embed_calls.load(Ordering::SeqCst),
        1,
        "batch embed must delegate to the inner provider"
    );
    assert_eq!(embed_one_calls.load(Ordering::SeqCst), 0);
    assert_eq!(cached.hits(), 0);
    assert_eq!(cached.misses(), 0);
}

#[tokio::test]
async fn distinct_queries_each_miss_the_cache() {
    let mock: Arc<dyn EmbeddingProvider> = Arc::new(MockProvider::default());
    let cached = Arc::new(CachingEmbeddingProvider::with_defaults(
        mock as Arc<dyn EmbeddingProvider>,
    ));

    let _ = cached.embed_one("query one").await.unwrap();
    let _ = cached.embed_one("query two").await.unwrap();
    let _ = cached.embed_one("query three").await.unwrap();

    assert_eq!(cached.misses(), 3, "three distinct queries must all miss");
    assert_eq!(cached.hits(), 0);
}
