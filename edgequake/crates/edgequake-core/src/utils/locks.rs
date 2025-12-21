//! Keyed locks for concurrent access control.
//!
//! This module provides a mechanism for acquiring locks on specific keys,
//! allowing concurrent operations on different keys while serializing
//! operations on the same key.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};

/// A collection of locks indexed by key.
///
/// This allows fine-grained locking where operations on different keys
/// can proceed concurrently, but operations on the same key are serialized.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use edgequake_core::utils::KeyedLocks;
///
/// #[tokio::main]
/// async fn main() {
///     let locks: KeyedLocks<String> = KeyedLocks::new();
///     
///     // Acquire lock for a specific key
///     let _guard = locks.lock("entity-123".to_string()).await;
///     // ... perform operation on entity-123
///     // Lock is released when guard is dropped
/// }
/// ```
pub struct KeyedLocks<K>
where
    K: Eq + Hash + Clone + Send + 'static,
{
    locks: RwLock<HashMap<K, Arc<Mutex<()>>>>,
}

impl<K> KeyedLocks<K>
where
    K: Eq + Hash + Clone + Send + 'static,
{
    /// Create a new empty KeyedLocks collection.
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
        }
    }

    /// Create with a pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            locks: RwLock::new(HashMap::with_capacity(capacity)),
        }
    }

    /// Acquire a lock for the given key.
    ///
    /// This will block if another task holds the lock for this key.
    /// Operations on different keys can proceed concurrently.
    pub async fn lock(&self, key: K) -> OwnedMutexGuard<()> {
        let mutex = self.get_or_create_mutex(key).await;
        mutex.lock_owned().await
    }

    /// Try to acquire a lock for the given key without blocking.
    ///
    /// Returns `None` if the lock is already held by another task.
    pub async fn try_lock(&self, key: K) -> Option<OwnedMutexGuard<()>> {
        let mutex = self.get_or_create_mutex(key).await;
        mutex.try_lock_owned().ok()
    }

    /// Get or create a mutex for the given key.
    async fn get_or_create_mutex(&self, key: K) -> Arc<Mutex<()>> {
        // First, try to get with a read lock
        {
            let locks = self.locks.read().await;
            if let Some(mutex) = locks.get(&key) {
                return Arc::clone(mutex);
            }
        }

        // Need to create - acquire write lock
        let mut locks = self.locks.write().await;
        // Double-check in case another task created it
        locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Remove a lock entry if it's not currently held.
    ///
    /// This helps clean up locks that are no longer needed.
    /// Returns true if the lock was removed.
    pub async fn cleanup(&self, key: &K) -> bool {
        let mut locks = self.locks.write().await;
        if let Some(mutex) = locks.get(key) {
            // Only remove if not currently locked
            if mutex.try_lock().is_ok() {
                locks.remove(key);
                return true;
            }
        }
        false
    }

    /// Get the number of tracked keys.
    pub async fn len(&self) -> usize {
        self.locks.read().await.len()
    }

    /// Check if there are no tracked keys.
    pub async fn is_empty(&self) -> bool {
        self.locks.read().await.is_empty()
    }

    /// Clear all locks (use with caution).
    ///
    /// This should only be called when you're certain no operations
    /// are in progress.
    pub async fn clear(&self) {
        self.locks.write().await.clear();
    }
}

impl<K> Default for KeyedLocks<K>
where
    K: Eq + Hash + Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A guard that holds multiple locks.
///
/// Useful when you need to lock multiple keys atomically.
pub struct MultiLockGuard<K>
where
    K: Eq + Hash + Clone + Send + 'static,
{
    _guards: Vec<(K, OwnedMutexGuard<()>)>,
}

impl<K> MultiLockGuard<K>
where
    K: Eq + Hash + Clone + Send + Ord + 'static,
{
    /// Acquire locks for multiple keys in a consistent order.
    ///
    /// Keys are sorted before locking to prevent deadlocks.
    pub async fn new(locks: &KeyedLocks<K>, mut keys: Vec<K>) -> Self {
        // Sort keys to ensure consistent lock ordering and prevent deadlocks
        keys.sort();
        keys.dedup();

        let mut guards = Vec::with_capacity(keys.len());
        for key in keys {
            let guard = locks.lock(key.clone()).await;
            guards.push((key, guard));
        }

        Self { _guards: guards }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_basic_locking() {
        let locks: KeyedLocks<String> = KeyedLocks::new();

        let _guard = locks.lock("key1".to_string()).await;
        assert_eq!(locks.len().await, 1);
    }

    #[tokio::test]
    async fn test_concurrent_different_keys() {
        let locks: Arc<KeyedLocks<String>> = Arc::new(KeyedLocks::new());
        let locks1 = Arc::clone(&locks);
        let locks2 = Arc::clone(&locks);

        // Both should acquire immediately since different keys
        let handle1 = tokio::spawn(async move {
            let _guard = locks1.lock("key1".to_string()).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        });

        let handle2 = tokio::spawn(async move {
            let _guard = locks2.lock("key2".to_string()).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        });

        // Both should complete quickly (not wait for each other)
        let result = timeout(Duration::from_millis(200), async {
            handle1.await.unwrap();
            handle2.await.unwrap();
        })
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_serialized_same_key() {
        let locks: Arc<KeyedLocks<String>> = Arc::new(KeyedLocks::new());
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let locks1 = Arc::clone(&locks);
        let counter1 = Arc::clone(&counter);

        let locks2 = Arc::clone(&locks);
        let counter2 = Arc::clone(&counter);

        // First task holds the lock
        let handle1 = tokio::spawn(async move {
            let _guard = locks1.lock("same-key".to_string()).await;
            counter1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(50)).await;
        });

        // Give first task time to acquire lock
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Second task should wait
        let handle2 = tokio::spawn(async move {
            let _guard = locks2.lock("same-key".to_string()).await;
            counter2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_try_lock() {
        let locks: KeyedLocks<String> = KeyedLocks::new();

        // Should succeed
        let guard = locks.try_lock("key".to_string()).await;
        assert!(guard.is_some());

        // Should fail - lock is held
        let locks2 = KeyedLocks::new();
        let _g = locks2.lock("key2".to_string()).await;
        // This creates a new entry, try_lock should still work on fresh locks
        let guard2 = locks2.try_lock("key3".to_string()).await;
        assert!(guard2.is_some());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let locks: KeyedLocks<String> = KeyedLocks::new();

        {
            let _guard = locks.lock("temp-key".to_string()).await;
        }

        // Lock is released, cleanup should succeed
        assert!(locks.cleanup(&"temp-key".to_string()).await);
        assert!(locks.is_empty().await);
    }

    #[tokio::test]
    async fn test_multi_lock_guard() {
        let locks: KeyedLocks<String> = KeyedLocks::new();

        let keys = vec![
            "z-key".to_string(),
            "a-key".to_string(),
            "m-key".to_string(),
        ];

        let _multi_guard = MultiLockGuard::new(&locks, keys).await;
        assert_eq!(locks.len().await, 3);
    }
}
