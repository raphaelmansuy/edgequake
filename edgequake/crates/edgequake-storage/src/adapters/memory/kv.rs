//! In-memory key-value storage.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
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
        let mut init = self.initialized.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        *init = true;
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn get_by_id<T: DeserializeOwned + Send>(&self, id: &str) -> Result<Option<T>> {
        let data = self.data.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        match data.get(id) {
            Some(value) => {
                let result = serde_json::from_value(value.clone())?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    async fn get_by_ids<T: DeserializeOwned + Send>(&self, ids: &[String]) -> Result<Vec<T>> {
        let data = self.data.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        let mut results = Vec::new();
        for id in ids {
            if let Some(value) = data.get(id) {
                if let Ok(item) = serde_json::from_value(value.clone()) {
                    results.push(item);
                }
            }
        }
        Ok(results)
    }

    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>> {
        let data = self.data.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        let missing: HashSet<String> = keys
            .into_iter()
            .filter(|k| !data.contains_key(k))
            .collect();
        Ok(missing)
    }

    async fn upsert<T: Serialize + Send + Sync>(&self, items: &[(String, T)]) -> Result<()> {
        let mut data = self.data.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        for (id, value) in items {
            let json_value = serde_json::to_value(value)?;
            data.insert(id.clone(), json_value);
        }
        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        let mut data = self.data.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        for id in ids {
            data.remove(id);
        }
        Ok(())
    }

    async fn is_empty(&self) -> Result<bool> {
        let data = self.data.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        Ok(data.is_empty())
    }

    async fn count(&self) -> Result<usize> {
        let data = self.data.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        Ok(data.len())
    }

    async fn keys(&self) -> Result<Vec<String>> {
        let data = self.data.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        Ok(data.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        data.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestItem {
        id: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_kv_basic_operations() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        // Insert
        let item = TestItem {
            id: "1".to_string(),
            value: 42,
        };
        storage
            .upsert(&[("1".to_string(), item.clone())])
            .await
            .unwrap();

        // Get
        let retrieved: Option<TestItem> = storage.get_by_id("1").await.unwrap();
        assert_eq!(retrieved, Some(item));

        // Get missing
        let missing: Option<TestItem> = storage.get_by_id("999").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_kv_batch_operations() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        let items: Vec<(String, TestItem)> = (0..5)
            .map(|i| {
                (
                    i.to_string(),
                    TestItem {
                        id: i.to_string(),
                        value: i,
                    },
                )
            })
            .collect();

        storage.upsert(&items).await.unwrap();

        let ids: Vec<String> = vec!["0".to_string(), "2".to_string(), "999".to_string()];
        let results: Vec<TestItem> = storage.get_by_ids(&ids).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_kv_filter_keys() {
        let storage = MemoryKVStorage::new("test");
        storage.initialize().await.unwrap();

        storage
            .upsert(&[("a".to_string(), 1), ("b".to_string(), 2)])
            .await
            .unwrap();

        let keys: HashSet<String> = ["a", "b", "c", "d"]
            .iter()
            .map(|s| s.to_string())
            .collect();
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
            .upsert(&[("1".to_string(), 1), ("2".to_string(), 2)])
            .await
            .unwrap();

        assert_eq!(storage.count().await.unwrap(), 2);

        storage.delete(&["1".to_string()]).await.unwrap();
        assert_eq!(storage.count().await.unwrap(), 1);

        let item: Option<i32> = storage.get_by_id("1").await.unwrap();
        assert!(item.is_none());
    }
}
