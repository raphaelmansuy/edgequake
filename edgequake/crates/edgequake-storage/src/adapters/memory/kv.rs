//! In-memory key-value storage.
//!
//! Thread-safe key-value storage for document and chunk metadata.
//!
//! ## Implements
//!
//! - [`FEAT0230`]: In-memory KV storage
//! - [`FEAT0231`]: Batch upsert operations
//! - [`FEAT0232`]: Key filtering for deduplication
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores document metadata
//! - [`UC0605`]: System retrieves chunks by ID
//!
//! ## Enforces
//!
//! - [`BR0230`]: Thread-safe concurrent access via RwLock
//! - [`BR0231`]: Atomic batch operations

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use crate::error::{Result, StorageError};
use crate::traits::KVStorage;

/// In-memory key-value storage implementation.
///
/// Thread-safe storage using `RwLock` for concurrent access.
/// Suitable for testing and development.
pub struct MemoryKVStorage {
    namespace: String,
    data: RwLock<HashMap<String, serde_json::Value>>,
    initialized: RwLock<bool>,
}

impl MemoryKVStorage {
    /// Create a new in-memory KV storage.
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            data: RwLock::new(HashMap::new()),
            initialized: RwLock::new(false),
        }
    }
}

#[async_trait]
impl KVStorage for MemoryKVStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn initialize(&self) -> Result<()> {
        let mut init = self
            .initialized
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        *init = true;
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let data = self
            .data
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        Ok(data.get(id).cloned())
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<serde_json::Value>> {
        let data = self
            .data
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        let mut results = Vec::new();
        for id in ids {
            if let Some(value) = data.get(id) {
                results.push(value.clone());
            }
        }
        Ok(results)
    }

    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>> {
        let data = self
            .data
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        let missing: HashSet<String> = keys.into_iter().filter(|k| !data.contains_key(k)).collect();
        Ok(missing)
    }

    async fn upsert(&self, items: &[(String, serde_json::Value)]) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        for (id, value) in items {
            data.insert(id.clone(), value.clone());
        }
        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        for id in ids {
            data.remove(id);
        }
        Ok(())
    }

    async fn is_empty(&self) -> Result<bool> {
        let data = self
            .data
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        Ok(data.is_empty())
    }

    async fn count(&self) -> Result<usize> {
        let data = self
            .data
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        Ok(data.len())
    }

    async fn keys(&self) -> Result<Vec<String>> {
        let data = self
            .data
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        Ok(data.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        data.clear();
        Ok(())
    }

    /// Atomically transition document status if current status matches expected.
    ///
    /// @implements FIX-RACE-01: Prevent TOCTOU race conditions
    ///
    /// # WHY: Memory-Safe Atomic Transition
    ///
    /// The write lock ensures atomicity - only one thread can check and update
    /// the status at a time. This prevents race conditions where:
    /// 1. Thread A reads status = "failed"
    /// 2. Thread B changes status to "processing"
    /// 3. Thread A thinks it can delete (based on stale read)
    ///
    /// With this method, the check and update are atomic within the lock.
    async fn transition_if_status(
        &self,
        key: &str,
        expected_status: &str,
        new_status: &str,
    ) -> Result<bool> {
        let mut data = self
            .data
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        // Check if key exists and status matches
        if let Some(value) = data.get_mut(key) {
            let current_status = value.get("status").and_then(|v| v.as_str());

            if current_status == Some(expected_status) {
                // Status matches - update it
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("status".to_string(), serde_json::json!(new_status));
                    return Ok(true);
                }
            }
        }

        // Key not found or status didn't match
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_kv_basic_operations() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        // Insert
        let item = json!({
            "id": "1",
            "value": 42
        });
        storage
            .upsert(&[("1".to_string(), item.clone())])
            .await
            .unwrap();

        // Get
        let retrieved = storage.get_by_id("1").await.unwrap();
        assert_eq!(retrieved, Some(item));

        // Get missing
        let missing = storage.get_by_id("999").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_kv_batch_operations() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        let items: Vec<(String, serde_json::Value)> = (0..5)
            .map(|i| {
                (
                    i.to_string(),
                    json!({
                        "id": i.to_string(),
                        "value": i
                    }),
                )
            })
            .collect();

        storage.upsert(&items).await.unwrap();

        let ids: Vec<String> = vec!["0".to_string(), "2".to_string(), "999".to_string()];
        let results = storage.get_by_ids(&ids).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_kv_filter_keys() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        storage
            .upsert(&[("a".to_string(), json!(1)), ("b".to_string(), json!(2))])
            .await
            .unwrap();

        let keys: HashSet<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let missing = storage.filter_keys(keys).await.unwrap();

        assert_eq!(missing.len(), 2);
        assert!(missing.contains("c"));
        assert!(missing.contains("d"));
    }

    #[tokio::test]
    async fn test_kv_delete() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        storage
            .upsert(&[("1".to_string(), json!(1)), ("2".to_string(), json!(2))])
            .await
            .unwrap();

        assert_eq!(storage.count().await.unwrap(), 2);

        storage.delete(&["1".to_string()]).await.unwrap();
        assert_eq!(storage.count().await.unwrap(), 1);

        let item = storage.get_by_id("1").await.unwrap();
        assert!(item.is_none());
    }

    /// @implements FIX-RACE-01: Test atomic status transition
    #[tokio::test]
    async fn test_transition_if_status_success() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        // Setup: document with status "failed"
        let doc = json!({
            "id": "doc-123",
            "status": "failed",
            "title": "Test Document"
        });
        storage
            .upsert(&[("doc-123-metadata".to_string(), doc)])
            .await
            .unwrap();

        // Action: transition from "failed" to "deleting"
        let result = storage
            .transition_if_status("doc-123-metadata", "failed", "deleting")
            .await
            .unwrap();

        // Assert: transition succeeded
        assert!(result, "Transition should succeed when status matches");

        // Verify: status is now "deleting"
        let updated = storage
            .get_by_id("doc-123-metadata")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.get("status").unwrap(), "deleting");
    }

    /// @implements FIX-RACE-01: Test atomic status transition fails on wrong status
    #[tokio::test]
    async fn test_transition_if_status_wrong_status() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        // Setup: document with status "processing"
        let doc = json!({
            "id": "doc-123",
            "status": "processing",
            "title": "Test Document"
        });
        storage
            .upsert(&[("doc-123-metadata".to_string(), doc)])
            .await
            .unwrap();

        // Action: try to transition from "failed" to "deleting" (wrong expected status)
        let result = storage
            .transition_if_status("doc-123-metadata", "failed", "deleting")
            .await
            .unwrap();

        // Assert: transition failed (status was "processing", not "failed")
        assert!(!result, "Transition should fail when status doesn't match");

        // Verify: status is still "processing"
        let unchanged = storage
            .get_by_id("doc-123-metadata")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.get("status").unwrap(), "processing");
    }

    /// @implements FIX-RACE-01: Test atomic status transition on non-existent key
    #[tokio::test]
    async fn test_transition_if_status_key_not_found() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        // Action: try to transition non-existent document
        let result = storage
            .transition_if_status("non-existent-key", "failed", "deleting")
            .await
            .unwrap();

        // Assert: transition failed (key doesn't exist)
        assert!(!result, "Transition should fail for non-existent key");
    }
}
