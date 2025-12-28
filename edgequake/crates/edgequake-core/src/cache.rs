//! Thread-safe LRU cache with TTL expiration.
//!
//! This module provides a generic cache implementation that combines
//! LRU eviction with time-based expiration.
//!
//! # Example
//!
//! ```rust
//! use edgequake_core::cache::TtlLruCache;
//! use std::time::Duration;
//!
//! let cache: TtlLruCache<String, String> = TtlLruCache::new(
//!     100,                      // Max 100 entries
//!     Duration::from_secs(300), // 5 minute TTL
//! );
//!
//! cache.put("key".to_string(), "value".to_string());
//! assert_eq!(cache.get(&"key".to_string()), Some("value".to_string()));
//! ```

use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// A cached value with expiration timestamp.
#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

/// Thread-safe LRU cache with TTL expiration.
///
/// # Type Parameters
///
/// - `K`: Key type (must be Clone + Eq + Hash)
/// - `V`: Value type (must be Clone)
///
/// # Thread Safety
///
/// The cache is thread-safe and can be shared across threads using `Arc`.
/// All operations use interior mutability with `RwLock`.
#[derive(Clone)]
pub struct TtlLruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    inner: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    ttl: Duration,

    // Metrics (optional, for monitoring)
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl<K, V> TtlLruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Create a new cache with specified capacity and TTL.
    ///
    /// # Arguments
    ///
    /// - `capacity`: Maximum number of entries
    /// - `ttl`: Time-to-live for entries
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(cap))),
            ttl,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get a value from the cache.
    ///
    /// Returns `None` if:
    /// - Key doesn't exist
    /// - Entry has expired (automatically removed)
    ///
    /// If entry exists and is valid, promotes it to most-recently-used.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().ok()?;

        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.value.clone());
            }
            // Entry expired - remove it
            cache.pop(key);
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Get a value without updating LRU order.
    ///
    /// Useful for checking existence without affecting eviction priority.
    pub fn peek(&self, key: &K) -> Option<V> {
        let cache = self.inner.read().ok()?;

        if let Some(entry) = cache.peek(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.value.clone());
            }
        }

        None
    }

    /// Insert a value into the cache.
    ///
    /// If the cache is at capacity, evicts the least-recently-used entry.
    /// Returns the previous value if key existed.
    pub fn put(&self, key: K, value: V) -> Option<V> {
        let mut cache = self.inner.write().ok()?;

        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };

        cache.put(key, entry).map(|e| e.value)
    }

    /// Remove a value from the cache.
    ///
    /// Returns the removed value if it existed.
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().ok()?;
        cache.pop(key).map(|e| e.value)
    }

    /// Invalidate (remove) an entry from the cache.
    ///
    /// Alias for `remove` with clearer intent.
    pub fn invalidate(&self, key: &K) {
        let _ = self.remove(key);
    }

    /// Get current cache size.
    pub fn len(&self) -> usize {
        self.inner.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.inner.write() {
            cache.clear();
        }
    }

    /// Get cache hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            hit_rate: self.hit_rate(),
        }
    }

    /// Remove all expired entries.
    ///
    /// This is O(n) and should be called periodically, not on every access.
    pub fn purge_expired(&self) {
        if let Ok(mut cache) = self.inner.write() {
            let now = Instant::now();
            let expired: Vec<K> = cache
                .iter()
                .filter(|(_, entry)| entry.expires_at <= now)
                .map(|(k, _)| k.clone())
                .collect();

            for key in expired {
                cache.pop(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), None);
    }

    #[test]
    fn test_lru_eviction() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            2, // Only 2 entries
            Duration::from_secs(60),
        );

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        // Access "a" to make it recently used
        cache.get(&"a".to_string());

        // Add "c" - should evict "b" (least recently used)
        cache.put("c".to_string(), 3);

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.get(&"b".to_string()), None); // Evicted
    }

    #[test]
    fn test_ttl_expiration() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            10,
            Duration::from_millis(50), // Very short TTL
        );

        cache.put("a".to_string(), 1);
        assert_eq!(cache.get(&"a".to_string()), Some(1));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(60));

        assert_eq!(cache.get(&"a".to_string()), None); // Expired
    }

    #[test]
    fn test_invalidation() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("a".to_string(), 1);
        assert_eq!(cache.get(&"a".to_string()), Some(1));

        cache.invalidate(&"a".to_string());
        assert_eq!(cache.get(&"a".to_string()), None);
    }

    #[test]
    fn test_hit_rate() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("a".to_string(), 1);

        // 2 hits
        cache.get(&"a".to_string());
        cache.get(&"a".to_string());

        // 1 miss
        cache.get(&"b".to_string());

        // 2/3 = 66.67%
        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_peek_does_not_update_lru() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            2, // Only 2 entries
            Duration::from_secs(60),
        );

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        // Peek "a" - should NOT update LRU order
        cache.peek(&"a".to_string());

        // Add "c" - should evict "a" (still LRU since peek doesn't update)
        cache.put("c".to_string(), 3);

        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.get(&"a".to_string()), None); // Evicted
    }

    #[test]
    fn test_clear() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_remove() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("a".to_string(), 1);

        let removed = cache.remove(&"a".to_string());
        assert_eq!(removed, Some(1));

        assert_eq!(cache.get(&"a".to_string()), None);
    }

    #[test]
    fn test_update_value() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("a".to_string(), 1);
        let old = cache.put("a".to_string(), 2);

        assert_eq!(old, Some(1));
        assert_eq!(cache.get(&"a".to_string()), Some(2));
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let cache = Arc::new(TtlLruCache::new(100, Duration::from_secs(60)));

        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = cache.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    cache_clone.put(format!("key_{}_{}", i, j), i * 100 + j);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Cache should not panic and should have some entries
        assert!(cache.len() <= 100);
    }
}
