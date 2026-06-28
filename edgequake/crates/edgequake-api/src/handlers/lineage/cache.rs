//! Lineage response cache (OODA-23).
//!
//! WHY: Lineage data rarely changes after document processing completes.
//! Caching avoids repeated KV lookups for the same document, providing
//! sub-millisecond response times for dashboard and UI polling scenarios.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::error::ApiError;

/// Cache TTL balances freshness vs. performance (T1: P95 < 200ms).
pub(super) const LINEAGE_CACHE_TTL: Duration = Duration::from_secs(120);

/// Maximum entries before evicting oldest. Prevents unbounded memory growth.
pub(super) const LINEAGE_CACHE_MAX_ENTRIES: usize = 500;

#[derive(Clone)]
pub(super) struct CachedLineage {
    pub(super) data: serde_json::Value,
    pub(super) cached_at: Instant,
}

pub(super) type LineageCache = Arc<RwLock<HashMap<String, CachedLineage>>>;

lazy_static::lazy_static! {
    pub(super) static ref LINEAGE_KV_CACHE: LineageCache = Arc::new(RwLock::new(HashMap::new()));
}

/// Read from lineage cache or fetch from KV storage.
///
/// WHY: Lineage queries hit KV storage on every request. After a document is
/// processed, the lineage data is immutable until reprocessing. Caching the
/// result avoids redundant I/O and meets the T1 latency target (<200ms P95).
pub(super) async fn cached_kv_get(
    kv: &dyn edgequake_storage::traits::KVStorage,
    key: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    // Check cache first
    {
        let cache = LINEAGE_KV_CACHE.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.cached_at.elapsed() < LINEAGE_CACHE_TTL {
                return Ok(Some(entry.data.clone()));
            }
        }
    }

    // Cache miss — fetch from storage
    let value = kv.get_by_id(key).await?;

    // Populate cache on hit
    if let Some(ref v) = value {
        let mut cache = LINEAGE_KV_CACHE.write().await;
        // WHY: Evict oldest entries when cache is full to prevent unbounded growth
        if cache.len() >= LINEAGE_CACHE_MAX_ENTRIES {
            // Simple eviction: remove entries older than TTL first
            cache.retain(|_, entry| entry.cached_at.elapsed() < LINEAGE_CACHE_TTL);
            // If still too full, clear half the cache
            if cache.len() >= LINEAGE_CACHE_MAX_ENTRIES {
                let keys_to_remove: Vec<String> =
                    cache.keys().take(cache.len() / 2).cloned().collect();
                for k in keys_to_remove {
                    cache.remove(&k);
                }
            }
        }
        cache.insert(
            key.to_string(),
            CachedLineage {
                data: v.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    Ok(value)
}

/// Invalidate a lineage cache entry.
///
/// WHY: Called after document reprocessing to ensure fresh data is served.
/// Without invalidation, stale lineage data would persist until TTL expires.
#[allow(dead_code)]
pub async fn invalidate_lineage_cache(document_id: &str) {
    let mut cache = LINEAGE_KV_CACHE.write().await;
    let lineage_key = format!("{}-lineage", document_id);
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(document_id);
    cache.remove(&lineage_key);
    cache.remove(&metadata_key);
    tracing::debug!(
        document_id = %document_id,
        "Invalidated lineage cache entries"
    );
}
