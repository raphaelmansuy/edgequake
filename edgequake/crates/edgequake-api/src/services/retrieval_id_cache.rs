//! TTL cache for retrieval_id → ContextRetrievalResponse (SPEC-028 MCP fetch).
//!
//! SPEC-146: bind each `ret_*` to principal (+ optional policy_generation);
//! fetch mismatch → 404 (existence-hiding).

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::handlers::context_types::ContextRetrievalResponse;

const DEFAULT_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_ENTRIES: usize = 500;

struct CacheEntry {
    response: ContextRetrievalResponse,
    expires_at: Instant,
    /// SPEC-146: owning principal (`user:…` / `apikey:…`). `None` = pre-146 / flag-off.
    principal: Option<String>,
    policy_generation: Option<u64>,
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
        self.store_for(response, None, None);
    }

    /// SPEC-146: store with principal binding.
    pub fn store_for(
        &self,
        response: ContextRetrievalResponse,
        principal: Option<String>,
        policy_generation: Option<u64>,
    ) {
        let retrieval_id = response.retrieval_id.clone();
        let entry = CacheEntry {
            response,
            expires_at: Instant::now() + self.ttl,
            principal,
            policy_generation,
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
        self.get_for(retrieval_id, None, None)
            .ok()
            .flatten()
    }

    /// SPEC-146: fetch with principal check. Mismatch → `Ok(None)` (404 at caller).
    /// When `expected_principal` is `None`, skip principal check (flag-off).
    pub fn get_for(
        &self,
        retrieval_id: &str,
        expected_principal: Option<&str>,
        expected_policy_generation: Option<u64>,
    ) -> Result<Option<ContextRetrievalResponse>, ()> {
        let guard = self.inner.read().expect("retrieval cache lock");
        let entry = match guard.get(retrieval_id) {
            Some(e) => e,
            None => return Ok(None),
        };
        if entry.expires_at <= Instant::now() {
            return Ok(None);
        }
        if let Some(expected) = expected_principal {
            match entry.principal.as_deref() {
                Some(bound) if bound == expected => {}
                Some(_) => return Err(()),
                // Bound entry without principal while caller expects one → deny.
                None if entry.principal.is_none() && expected_policy_generation.is_some() => {
                    return Err(());
                }
                None => {}
            }
        }
        if let (Some(exp_gen), Some(bound_gen)) =
            (expected_policy_generation, entry.policy_generation)
        {
            if exp_gen != bound_gen {
                return Err(());
            }
        }
        Ok(Some(entry.response.clone()))
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
