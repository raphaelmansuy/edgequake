//! In-memory vector storage.
//!
//! Provides vector storage using brute-force cosine similarity search.
//!
//! ## Implements
//!
//! - [`FEAT0220`]: In-memory vector storage
//! - [`FEAT0221`]: Cosine similarity search
//! - [`FEAT0222`]: Vector dimension validation
//!
//! ## Use Cases
//!
//! - [`UC0603`]: System performs vector similarity search
//! - [`UC0604`]: System retrieves similar chunks
//!
//! ## Enforces
//!
//! - [`BR0220`]: Dimension consistency validation
//! - [`BR0221`]: Thread-safe concurrent access via RwLock

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::error::{Result, StorageError};
use crate::traits::{VectorSearchResult, VectorStorage};

/// In-memory vector storage implementation.
///
/// Uses brute-force cosine similarity search.
/// Suitable for testing and small datasets.
pub struct MemoryVectorStorage {
    namespace: String,
    dimension: usize,
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    metadata: RwLock<HashMap<String, serde_json::Value>>,
}

impl MemoryVectorStorage {
    /// Create a new in-memory vector storage.
    pub fn new(namespace: impl Into<String>, dimension: usize) -> Self {
        Self {
            namespace: namespace.into(),
            dimension,
            vectors: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }
}

#[async_trait]
impl VectorStorage for MemoryVectorStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_embedding.len() != self.dimension {
            return Err(StorageError::InvalidQuery(format!(
                "Query dimension {} doesn't match expected {}",
                query_embedding.len(),
                self.dimension
            )));
        }

        let vectors = self
            .vectors
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        let metadata = self
            .metadata
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        let filter_set: Option<std::collections::HashSet<&String>> =
            filter_ids.map(|ids| ids.iter().collect());

        let mut scores: Vec<(String, f32)> = vectors
            .iter()
            .filter(|(id, _)| {
                filter_set
                    .as_ref()
                    .map(|set| set.contains(id))
                    .unwrap_or(true)
            })
            .map(|(id, vec)| {
                let score = Self::cosine_similarity(query_embedding, vec);
                (id.clone(), score)
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top_k
        let results: Vec<VectorSearchResult> = scores
            .into_iter()
            .take(top_k)
            .map(|(id, score)| VectorSearchResult {
                id: id.clone(),
                score,
                metadata: metadata
                    .get(&id)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            })
            .collect();

        Ok(results)
    }

    async fn upsert(&self, data: &[(String, Vec<f32>, serde_json::Value)]) -> Result<()> {
        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        let mut metadata = self
            .metadata
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        for (id, vec, meta) in data {
            if vec.len() != self.dimension {
                return Err(StorageError::InvalidQuery(format!(
                    "Vector dimension {} doesn't match expected {}",
                    vec.len(),
                    self.dimension
                )));
            }
            vectors.insert(id.clone(), vec.clone());
            metadata.insert(id.clone(), meta.clone());
        }

        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        let mut metadata = self
            .metadata
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        for id in ids {
            vectors.remove(id);
            metadata.remove(id);
        }

        Ok(())
    }

    async fn delete_entity(&self, entity_name: &str) -> Result<()> {
        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        let mut metadata = self
            .metadata
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        let to_remove: Vec<String> = vectors
            .keys()
            .filter(|k| k.contains(entity_name))
            .cloned()
            .collect();

        for id in to_remove {
            vectors.remove(&id);
            metadata.remove(&id);
        }

        Ok(())
    }

    async fn delete_entity_relations(&self, entity_name: &str) -> Result<()> {
        self.delete_entity(entity_name).await
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let vectors = self
            .vectors
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        Ok(vectors.get(id).cloned())
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>> {
        let vectors = self
            .vectors
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        let results: Vec<(String, Vec<f32>)> = ids
            .iter()
            .filter_map(|id| vectors.get(id).map(|v| (id.clone(), v.clone())))
            .collect();

        Ok(results)
    }

    async fn is_empty(&self) -> Result<bool> {
        let vectors = self
            .vectors
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        Ok(vectors.is_empty())
    }

    async fn count(&self) -> Result<usize> {
        let vectors = self
            .vectors
            .read()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        Ok(vectors.len())
    }

    async fn clear(&self) -> Result<()> {
        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        let mut metadata = self
            .metadata
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        vectors.clear();
        metadata.clear();
        Ok(())
    }

    /// Clear only vectors belonging to a specific workspace.
    ///
    /// Filters by `workspace_id` field in metadata JSON.
    /// Returns the count of deleted vectors.
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;
        let mut metadata_map = self
            .metadata
            .write()
            .map_err(|e| StorageError::Database(format!("Lock error: {}", e)))?;

        let workspace_id_str = workspace_id.to_string();

        // Collect keys to remove (matching workspace_id in metadata)
        let keys_to_remove: Vec<String> = metadata_map
            .iter()
            .filter_map(|(key, meta)| {
                if let Some(ws_id) = meta.get("workspace_id").and_then(|v| v.as_str()) {
                    if ws_id == workspace_id_str {
                        return Some(key.clone());
                    }
                }
                None
            })
            .collect();

        let count = keys_to_remove.len();

        // Remove from both vectors and metadata
        for key in keys_to_remove {
            vectors.remove(&key);
            metadata_map.remove(&key);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_basic_operations() {
        let storage = MemoryVectorStorage::new("test", 3);
        storage.initialize().await.unwrap();

        // Insert vectors
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"name": "a"}),
            ),
            (
                "b".to_string(),
                vec![0.0, 1.0, 0.0],
                serde_json::json!({"name": "b"}),
            ),
            (
                "c".to_string(),
                vec![0.0, 0.0, 1.0],
                serde_json::json!({"name": "c"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        assert_eq!(storage.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_vector_similarity_search() {
        let storage = MemoryVectorStorage::new("test", 3);
        storage.initialize().await.unwrap();

        let data = vec![
            ("a".to_string(), vec![1.0, 0.0, 0.0], serde_json::json!({})),
            ("b".to_string(), vec![0.9, 0.1, 0.0], serde_json::json!({})),
            ("c".to_string(), vec![0.0, 1.0, 0.0], serde_json::json!({})),
        ];
        storage.upsert(&data).await.unwrap();

        // Query similar to "a"
        let results = storage.query(&[1.0, 0.0, 0.0], 2, None).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a"); // Exact match
        assert_eq!(results[1].id, "b"); // Most similar
    }

    #[tokio::test]
    async fn test_vector_filtered_search() {
        let storage = MemoryVectorStorage::new("test", 3);

        let data = vec![
            ("a".to_string(), vec![1.0, 0.0, 0.0], serde_json::json!({})),
            ("b".to_string(), vec![0.9, 0.1, 0.0], serde_json::json!({})),
            ("c".to_string(), vec![0.0, 1.0, 0.0], serde_json::json!({})),
        ];
        storage.upsert(&data).await.unwrap();

        // Query with filter
        let filter = vec!["b".to_string(), "c".to_string()];
        let results = storage
            .query(&[1.0, 0.0, 0.0], 10, Some(&filter))
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(!results.iter().any(|r| r.id == "a"));
    }
}
