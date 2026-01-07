//! LLM Response Caching.
//!
//! Provides caching for LLM extraction responses to avoid redundant API calls.
//! Supports both in-memory caching and persistent storage.
//!
//! # Cache Key Generation
//!
//! Cache keys are generated from:
//! - Chunk content hash
//! - Model name
//! - Prompt version/hash
//!
//! # Usage
//!
//! ```rust,ignore
//! use edgequake_pipeline::cache::{LLMCache, MemoryLLMCache, CacheEntry};
//!
//! let cache = MemoryLLMCache::new();
//! let entry = cache.get("prompt_hash").await?;
//!
//! if let Some(cached) = entry {
//!     // Use cached response
//! } else {
//!     // Call LLM and cache result
//!     cache.set(CacheEntry { ... }).await?;
//! }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::Result;

/// Type of cached LLM response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheType {
    /// Entity extraction response.
    Extract,
    /// Gleaning (re-extraction) response.
    Glean,
    /// Summarization response.
    Summary,
    /// Embedding response.
    Embedding,
}

/// A cached LLM response entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Unique cache entry ID.
    pub id: String,
    /// Type of cached response.
    pub cache_type: CacheType,
    /// Associated chunk ID (if applicable).
    pub chunk_id: Option<String>,
    /// Hash of the prompt used.
    pub prompt_hash: String,
    /// The cached response content.
    pub response: String,
    /// Number of input tokens.
    pub input_tokens: usize,
    /// Number of output tokens.
    pub output_tokens: usize,
    /// Model used for generation.
    pub model: String,
    /// When the entry was created.
    pub created_at: DateTime<Utc>,
    /// Time to live in seconds (None = forever).
    pub ttl_seconds: Option<u64>,
}

impl CacheEntry {
    /// Create a new cache entry.
    pub fn new(
        cache_type: CacheType,
        prompt_hash: impl Into<String>,
        response: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            cache_type,
            chunk_id: None,
            prompt_hash: prompt_hash.into(),
            response: response.into(),
            input_tokens: 0,
            output_tokens: 0,
            model: model.into(),
            created_at: Utc::now(),
            ttl_seconds: None,
        }
    }

    /// Set the chunk ID.
    pub fn with_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        self.chunk_id = Some(chunk_id.into());
        self
    }

    /// Set token usage.
    pub fn with_token_usage(mut self, input: usize, output: usize) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    /// Set TTL.
    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }

    /// Check if the entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_seconds {
            let expiry = self.created_at + chrono::Duration::seconds(ttl as i64);
            Utc::now() > expiry
        } else {
            false
        }
    }
}

/// Generate a cache key from prompt and model.
pub fn generate_cache_key(prompt: &str, model: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    hasher.update(model.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a cache key from multiple inputs.
pub fn generate_cache_key_multi(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Trait for LLM response caching.
#[async_trait]
pub trait LLMCache: Send + Sync {
    /// Get a cached response by prompt hash.
    async fn get(&self, prompt_hash: &str) -> Result<Option<CacheEntry>>;

    /// Store a response in the cache.
    async fn set(&self, entry: CacheEntry) -> Result<()>;

    /// Get all cache entries for a chunk.
    async fn get_by_chunk(&self, chunk_id: &str) -> Result<Vec<CacheEntry>>;

    /// Delete cache entries by chunk ID.
    async fn delete_by_chunk(&self, chunk_id: &str) -> Result<usize>;

    /// Clear all cache entries.
    async fn clear(&self) -> Result<()>;

    /// Get cache statistics.
    async fn stats(&self) -> CacheStats;
}

/// Cache statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of entries.
    pub total_entries: usize,
    /// Number of extraction entries.
    pub extract_entries: usize,
    /// Number of gleaning entries.
    pub glean_entries: usize,
    /// Number of summary entries.
    pub summary_entries: usize,
    /// Total cached tokens (input + output).
    pub total_tokens: usize,
    /// Estimated cost savings (USD).
    pub estimated_savings_usd: f64,
}

/// In-memory LLM cache implementation.
#[derive(Debug)]
pub struct MemoryLLMCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    chunk_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl MemoryLLMCache {
    /// Create a new in-memory cache.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            chunk_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryLLMCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMCache for MemoryLLMCache {
    async fn get(&self, prompt_hash: &str) -> Result<Option<CacheEntry>> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(prompt_hash) {
            if entry.is_expired() {
                return Ok(None);
            }
            return Ok(Some(entry.clone()));
        }
        Ok(None)
    }

    async fn set(&self, entry: CacheEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;

        // Update chunk index
        if let Some(chunk_id) = &entry.chunk_id {
            chunk_index
                .entry(chunk_id.clone())
                .or_default()
                .push(entry.prompt_hash.clone());
        }

        entries.insert(entry.prompt_hash.clone(), entry);
        Ok(())
    }

    async fn get_by_chunk(&self, chunk_id: &str) -> Result<Vec<CacheEntry>> {
        let entries = self.entries.read().await;
        let chunk_index = self.chunk_index.read().await;

        let mut results = Vec::new();
        if let Some(hashes) = chunk_index.get(chunk_id) {
            for hash in hashes {
                if let Some(entry) = entries.get(hash) {
                    if !entry.is_expired() {
                        results.push(entry.clone());
                    }
                }
            }
        }
        Ok(results)
    }

    async fn delete_by_chunk(&self, chunk_id: &str) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;

        let hashes = chunk_index.remove(chunk_id).unwrap_or_default();
        let count = hashes.len();

        for hash in hashes {
            entries.remove(&hash);
        }

        Ok(count)
    }

    async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;

        entries.clear();
        chunk_index.clear();

        Ok(())
    }

    async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;

        let mut stats = CacheStats {
            total_entries: entries.len(),
            ..Default::default()
        };

        for entry in entries.values() {
            match entry.cache_type {
                CacheType::Extract => stats.extract_entries += 1,
                CacheType::Glean => stats.glean_entries += 1,
                CacheType::Summary => stats.summary_entries += 1,
                CacheType::Embedding => {}
            }
            stats.total_tokens += entry.input_tokens + entry.output_tokens;
        }

        // Estimate savings based on gpt-4o-mini pricing
        let input_cost_per_1k = 0.00015;
        let output_cost_per_1k = 0.0006;
        stats.estimated_savings_usd =
            (stats.total_tokens as f64 / 1000.0) * ((input_cost_per_1k + output_cost_per_1k) / 2.0);

        stats
    }
}

/// Cached extractor wrapper that checks cache before calling LLM.
pub struct CachedExtractor<E, C>
where
    E: crate::extractor::EntityExtractor,
    C: LLMCache,
{
    extractor: Arc<E>,
    cache: Arc<C>,
    model: String,
}

impl<E, C> CachedExtractor<E, C>
where
    E: crate::extractor::EntityExtractor,
    C: LLMCache,
{
    /// Create a new cached extractor.
    pub fn new(extractor: Arc<E>, cache: Arc<C>, model: impl Into<String>) -> Self {
        Self {
            extractor,
            cache,
            model: model.into(),
        }
    }
}

#[async_trait]
impl<E, C> crate::extractor::EntityExtractor for CachedExtractor<E, C>
where
    E: crate::extractor::EntityExtractor + Send + Sync,
    C: LLMCache + Send + Sync,
{
    async fn extract(
        &self,
        chunk: &crate::chunker::TextChunk,
    ) -> Result<crate::extractor::ExtractionResult> {
        let cache_key = generate_cache_key(&chunk.content, &self.model);

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key).await? {
            tracing::debug!(chunk_id = %chunk.id, "Cache hit for extraction");

            // Parse cached response
            let parser = crate::prompts::HybridExtractionParser::new(true);
            return parser.parse(&cached.response, &chunk.id);
        }

        // Cache miss - call extractor
        tracing::debug!(chunk_id = %chunk.id, "Cache miss for extraction");
        let result = self.extractor.extract(chunk).await?;

        // TODO: Store raw response in cache (would need to modify extractor to return it)
        // For now, we skip caching since we don't have access to the raw LLM response

        Ok(result)
    }

    fn name(&self) -> &str {
        "cached"
    }

    fn model_name(&self) -> &str {
        self.extractor.model_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key1 = generate_cache_key("prompt1", "model1");
        let key2 = generate_cache_key("prompt1", "model1");
        let key3 = generate_cache_key("prompt2", "model1");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new(CacheType::Extract, "hash123", "response", "gpt-4o-mini")
            .with_chunk_id("chunk-1")
            .with_token_usage(100, 50)
            .with_ttl(3600);

        assert_eq!(entry.cache_type, CacheType::Extract);
        assert_eq!(entry.chunk_id, Some("chunk-1".to_string()));
        assert_eq!(entry.input_tokens, 100);
        assert_eq!(entry.output_tokens, 50);
        assert_eq!(entry.ttl_seconds, Some(3600));
        assert!(!entry.is_expired());
    }

    #[tokio::test]
    async fn test_memory_cache_basic() {
        let cache = MemoryLLMCache::new();

        let entry = CacheEntry::new(CacheType::Extract, "hash1", "response1", "model1");
        cache.set(entry).await.unwrap();

        let retrieved = cache.get("hash1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().response, "response1");

        let missing = cache.get("hash2").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_chunk_index() {
        let cache = MemoryLLMCache::new();

        let entry1 = CacheEntry::new(CacheType::Extract, "hash1", "response1", "model1")
            .with_chunk_id("chunk-1");
        let entry2 = CacheEntry::new(CacheType::Glean, "hash2", "response2", "model1")
            .with_chunk_id("chunk-1");

        cache.set(entry1).await.unwrap();
        cache.set(entry2).await.unwrap();

        let chunk_entries = cache.get_by_chunk("chunk-1").await.unwrap();
        assert_eq!(chunk_entries.len(), 2);

        let deleted = cache.delete_by_chunk("chunk-1").await.unwrap();
        assert_eq!(deleted, 2);

        let after_delete = cache.get_by_chunk("chunk-1").await.unwrap();
        assert!(after_delete.is_empty());
    }

    #[tokio::test]
    async fn test_memory_cache_stats() {
        let cache = MemoryLLMCache::new();

        let entry1 = CacheEntry::new(CacheType::Extract, "hash1", "response1", "model1")
            .with_token_usage(100, 50);
        let entry2 = CacheEntry::new(CacheType::Glean, "hash2", "response2", "model1")
            .with_token_usage(200, 100);

        cache.set(entry1).await.unwrap();
        cache.set(entry2).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.extract_entries, 1);
        assert_eq!(stats.glean_entries, 1);
        assert_eq!(stats.total_tokens, 450);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = MemoryLLMCache::new();

        cache
            .set(CacheEntry::new(
                CacheType::Extract,
                "hash1",
                "response1",
                "model1",
            ))
            .await
            .unwrap();

        assert!(cache.get("hash1").await.unwrap().is_some());

        cache.clear().await.unwrap();

        assert!(cache.get("hash1").await.unwrap().is_none());
    }
}
