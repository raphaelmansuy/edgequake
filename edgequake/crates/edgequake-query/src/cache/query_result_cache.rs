//! Query-result cache for `context_only` retrieval (P-G9 / RC-14).
//!
//! Caches enriched `QueryContext` keyed by `(query, mode, allowed_document_ids)` so
//! repeated identical inspection queries skip retrieval. Does NOT cache generated
//! answers (non-deterministic / conversation-dependent).

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::context::QueryContext;
use crate::modes::QueryMode;
use crate::types::QueryRequest;

fn cache_key(request: &QueryRequest, mode: QueryMode) -> String {
    let mut hasher = DefaultHasher::new();
    request.query.hash(&mut hasher);
    format!("{:?}", mode).hash(&mut hasher);
    if let Some(ids) = &request.allowed_document_ids {
        for id in ids {
            id.hash(&mut hasher);
        }
    }
    format!("ctx:{:x}", hasher.finish())
}

struct Entry {
    context: QueryContext,
    expires_at: Option<Instant>,
}

/// LRU + TTL cache for `context_only` query contexts.
pub struct QueryResultCache {
    max_size: usize,
    ttl: Duration,
    epoch: RwLock<u64>,
    cache: RwLock<HashMap<String, Entry>>,
    hits: RwLock<u64>,
    misses: RwLock<u64>,
}

impl QueryResultCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            epoch: RwLock::new(0),
            cache: RwLock::new(HashMap::new()),
            hits: RwLock::new(0),
            misses: RwLock::new(0),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(1_000, Duration::from_secs(300))
    }

    /// Bump workspace/global epoch — entries stamped before bump are evicted on read.
    pub fn bump_epoch(&self) {
        let mut epoch = self.epoch.write().expect("epoch lock");
        *epoch += 1;
        self.cache.write().expect("cache lock").clear();
    }

    /// Alias for ingestion hooks (P-G9 invalidation API).
    pub fn invalidate_all(&self) {
        self.bump_epoch();
    }

    pub fn hits(&self) -> u64 {
        *self.hits.read().expect("hits lock")
    }

    pub fn misses(&self) -> u64 {
        *self.misses.read().expect("misses lock")
    }

    pub fn get(&self, request: &QueryRequest, mode: QueryMode) -> Option<QueryContext> {
        if !request.context_only {
            return None;
        }
        let key = cache_key(request, mode);
        let now = Instant::now();
        let mut cache = self.cache.write().expect("cache lock");
        let entry = cache.get(&key)?;
        if let Some(expires) = entry.expires_at {
            if now >= expires {
                cache.remove(&key);
                *self.misses.write().expect("misses lock") += 1;
                return None;
            }
        }
        *self.hits.write().expect("hits lock") += 1;
        Some(entry.context.clone())
    }

    pub fn put(&self, request: &QueryRequest, mode: QueryMode, context: QueryContext) {
        if !request.context_only {
            return;
        }
        let key = cache_key(request, mode);
        let expires_at = Some(Instant::now() + self.ttl);
        let mut cache = self.cache.write().expect("cache lock");
        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            if let Some(old_key) = cache.keys().next().cloned() {
                cache.remove(&old_key);
            }
        }
        cache.insert(key, Entry { context, expires_at });
    }

    pub fn record_miss(&self) {
        *self.misses.write().expect("misses lock") += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_context_only_query_hits_cache() {
        let cache = QueryResultCache::with_defaults();
        let mut req = QueryRequest::new("what is edgequake?");
        req.context_only = true;

        let ctx = QueryContext::default();
        cache.put(&req, QueryMode::Hybrid, ctx.clone());
        assert!(cache.get(&req, QueryMode::Hybrid).is_some());
        assert_eq!(cache.hits(), 1);

        assert!(cache.get(&req, QueryMode::Hybrid).is_some());
        assert_eq!(cache.hits(), 2);
    }

    #[test]
    fn bump_epoch_evicts_entries() {
        let cache = QueryResultCache::with_defaults();
        let mut req = QueryRequest::new("test");
        req.context_only = true;
        cache.put(&req, QueryMode::Local, QueryContext::default());
        cache.bump_epoch();
        assert!(cache.get(&req, QueryMode::Local).is_none());
    }
}
