//! TTL cache for retrieval_id → ContextRetrievalResponse (SPEC-028 MCP fetch).

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::handlers::context_types::ContextRetrievalResponse;

const DEFAULT_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_ENTRIES: usize = 500;

struct CacheEntry {
    response: ContextRetrievalResponse,
    expires_at: Instant,
}

/// In-memory retrieval handle cache (stateless MCP fetch SSOT).
#[derive(Default)]
pub struct RetrievalIdCache {
    inner: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl RetrievalIdCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            ttl: DEFAULT_TTL,
        }
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    pub fn store(&self, response: ContextRetrievalResponse) {
        let retrieval_id = response.retrieval_id.clone();
        let entry = CacheEntry {
            response,
            expires_at: Instant::now() + self.ttl,
        };

        let mut guard = self.inner.write().expect("retrieval cache lock");
        if guard.len() >= MAX_ENTRIES {
            let now = Instant::now();
            guard.retain(|_, e| e.expires_at > now);
            if guard.len() >= MAX_ENTRIES {
                guard.clear();
            }
        }
        guard.insert(retrieval_id, entry);
    }

    pub fn get(&self, retrieval_id: &str) -> Option<ContextRetrievalResponse> {
        let guard = self.inner.read().expect("retrieval cache lock");
        let entry = guard.get(retrieval_id)?;
        if entry.expires_at <= Instant::now() {
            return None;
        }
        Some(entry.response.clone())
    }

    pub fn is_expired(&self, retrieval_id: &str) -> bool {
        let guard = self.inner.read().expect("retrieval cache lock");
        match guard.get(retrieval_id) {
            Some(entry) => entry.expires_at <= Instant::now(),
            None => false,
        }
    }

    /// Test-only: force a cache entry to appear expired (EC-MCP-27 e2e).
    #[doc(hidden)]
    pub fn expire_entry_for_test(&self, retrieval_id: &str) {
        let mut guard = self.inner.write().expect("retrieval cache lock");
        if let Some(entry) = guard.get_mut(retrieval_id) {
            entry.expires_at = Instant::now() - Duration::from_secs(1);
        }
    }
}

pub fn global_retrieval_cache() -> &'static RetrievalIdCache {
    static CACHE: std::sync::OnceLock<RetrievalIdCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(RetrievalIdCache::new)
}

pub fn new_retrieval_id() -> String {
    format!("ret_{}", uuid::Uuid::new_v4())
}
