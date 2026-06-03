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
use crate::traits::{MetadataFilter, VectorSearchResult, VectorStorage};

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

        let vectors = self.vectors.read().map_err(super::lock::map_lock_err)?;
        let metadata = self.metadata.read().map_err(super::lock::map_lock_err)?;

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
        let mut vectors = self.vectors.write().map_err(super::lock::map_lock_err)?;
        let mut metadata = self.metadata.write().map_err(super::lock::map_lock_err)?;

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
        let mut vectors = self.vectors.write().map_err(super::lock::map_lock_err)?;
        let mut metadata = self.metadata.write().map_err(super::lock::map_lock_err)?;

        for id in ids {
            vectors.remove(id);
            metadata.remove(id);
        }

        Ok(())
    }

    async fn delete_entity(&self, entity_name: &str) -> Result<()> {
        let mut vectors = self.vectors.write().map_err(super::lock::map_lock_err)?;
        let mut metadata = self.metadata.write().map_err(super::lock::map_lock_err)?;

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
        let vectors = self.vectors.read().map_err(super::lock::map_lock_err)?;
        Ok(vectors.get(id).cloned())
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>> {
        let vectors = self.vectors.read().map_err(super::lock::map_lock_err)?;

        let results: Vec<(String, Vec<f32>)> = ids
            .iter()
            .filter_map(|id| vectors.get(id).map(|v| (id.clone(), v.clone())))
            .collect();

        Ok(results)
    }

    async fn is_empty(&self) -> Result<bool> {
        let vectors = self.vectors.read().map_err(super::lock::map_lock_err)?;
        Ok(vectors.is_empty())
    }

    async fn count(&self) -> Result<usize> {
        let vectors = self.vectors.read().map_err(super::lock::map_lock_err)?;
        Ok(vectors.len())
    }

    async fn clear(&self) -> Result<()> {
        let mut vectors = self.vectors.write().map_err(super::lock::map_lock_err)?;
        let mut metadata = self.metadata.write().map_err(super::lock::map_lock_err)?;

        vectors.clear();
        metadata.clear();
        Ok(())
    }

    /// Clear only vectors belonging to a specific workspace.
    ///
    /// Filters by `workspace_id` field in metadata JSON.
    /// Returns the count of deleted vectors.
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let mut vectors = self.vectors.write().map_err(super::lock::map_lock_err)?;
        let mut metadata_map = self.metadata.write().map_err(super::lock::map_lock_err)?;

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

    /// Query with metadata pre-filter (SPEC-007 Tier 2).
    ///
    /// Applies MetadataFilter conditions in-memory, matching the same semantics
    /// as the SQL-level filter in PgVectorStorage.
    ///
    /// @implements SPEC-007 R-T2-01
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // Fast path: no metadata filter → delegate to standard query
        let mf = match metadata_filter {
            Some(mf) if !mf.is_empty() => mf,
            _ => return self.query(query_embedding, top_k, filter_ids).await,
        };

        if query_embedding.len() != self.dimension {
            return Err(StorageError::InvalidQuery(format!(
                "Query dimension {} doesn't match expected {}",
                query_embedding.len(),
                self.dimension
            )));
        }

        let vectors = self.vectors.read().map_err(super::lock::map_lock_err)?;
        let metadata = self.metadata.read().map_err(super::lock::map_lock_err)?;

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
            .filter(|(id, _)| {
                let meta = match metadata.get(*id) {
                    Some(m) => m,
                    None => return false,
                };
                mf.matches(meta)
            })
            .map(|(id, vec)| {
                let score = Self::cosine_similarity(query_embedding, vec);
                (id.clone(), score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

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

    // --- SPEC-007 MetadataFilter integration tests ---

    #[tokio::test]
    async fn test_query_filtered_no_filter_delegates_to_query() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        // None filter returns all
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_query_filtered_by_tenant_id() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.id == "a" || r.id == "c"));
    }

    #[tokio::test]
    async fn test_query_filtered_by_workspace_id() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"workspace_id": "ws1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"workspace_id": "ws2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            workspace_id: Some("ws1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn test_query_filtered_by_document_ids() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"document_id": "doc1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"document_id": "doc2"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"source_document_id": "doc1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            document_ids: Some(vec!["doc1".to_string()]),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        // Should match "a" (document_id) and "c" (source_document_id)
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.id == "a" || r.id == "c"));
    }

    #[tokio::test]
    async fn test_query_filtered_combined_tenant_and_workspace() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1", "workspace_id": "ws1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t1", "workspace_id": "ws2"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t2", "workspace_id": "ws1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            workspace_id: Some("ws1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn test_query_filtered_with_filter_ids_and_metadata_filter() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        // Both filter_ids AND MetadataFilter should be combined (AND)
        let filter_ids = vec!["a".to_string(), "b".to_string()];
        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, Some(&filter_ids), Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.id == "a" || r.id == "b"));
    }

    #[tokio::test]
    async fn test_query_filtered_empty_filter_returns_all() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        // Empty MetadataFilter should return all results
        let mf = MetadataFilter::default();
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_query_filtered_top_k_applied_after_filter() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "d".to_string(),
                vec![0.7, 0.3, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        // Request top_k=2 — should get 2 of the 3 matching t1 vectors
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 2, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        // Should be the two highest-scoring t1 vectors
        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "b");
    }

    // --- SPEC-007 edge-case tests ---

    #[tokio::test]
    async fn test_query_filtered_no_metadata_on_record_excluded() {
        // Vectors with null metadata: tenant/workspace filters pass (lenient),
        // but document_ids filter excludes (strict, no matching key found)
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "has_meta".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"document_id": "doc1", "tenant_id": "t1"}),
            ),
            (
                "null_meta".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!(null),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        // Tenant filter: null metadata passes (lenient)
        let mf_tenant = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf_tenant))
            .await
            .unwrap();
        assert_eq!(results.len(), 2); // both pass tenant filter

        // Document IDs filter: null metadata excluded (strict)
        let mf_doc = MetadataFilter {
            document_ids: Some(vec!["doc1".to_string()]),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf_doc))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "has_meta");
    }

    #[tokio::test]
    async fn test_query_filtered_missing_tenant_field_in_metadata_passes() {
        // Lenient: if record metadata has no "tenant_id" key, tenant filter passes
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "has_tenant".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "no_tenant".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"workspace_id": "ws1"}),
            ),
            (
                "wrong_tenant".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        // "has_tenant" matches, "no_tenant" passes (lenient), "wrong_tenant" excluded
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"has_tenant"));
        assert!(ids.contains(&"no_tenant"));
    }

    #[tokio::test]
    async fn test_query_filtered_missing_workspace_field_in_metadata_passes() {
        // Lenient: if record metadata has no "workspace_id" key, workspace filter passes
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "has_ws".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"workspace_id": "ws1"}),
            ),
            (
                "no_ws".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "wrong_ws".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"workspace_id": "ws2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            workspace_id: Some("ws1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        // "has_ws" matches, "no_ws" passes (lenient), "wrong_ws" excluded
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"has_ws"));
        assert!(ids.contains(&"no_ws"));
    }

    #[tokio::test]
    async fn test_query_filtered_missing_document_id_field_excluded() {
        // Strict: if record has no document_id/source_document_id, document_ids filter EXCLUDES it
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "has_doc".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"document_id": "doc1"}),
            ),
            (
                "no_doc".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            document_ids: Some(vec!["doc1".to_string()]),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "has_doc");
    }

    #[tokio::test]
    async fn test_query_filtered_empty_document_ids_matches_none() {
        // Empty document_ids vec should match nothing
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![(
            "a".to_string(),
            vec![1.0, 0.0, 0.0],
            serde_json::json!({"document_id": "doc1"}),
        )];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            document_ids: Some(vec![]),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_query_filtered_multiple_document_ids() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"document_id": "doc1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"document_id": "doc2"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"document_id": "doc3"}),
            ),
            (
                "d".to_string(),
                vec![0.7, 0.3, 0.0],
                serde_json::json!({"source_document_id": "doc1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            document_ids: Some(vec!["doc1".to_string(), "doc2".to_string()]),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        // "a" (doc1), "b" (doc2), "d" (source_document_id: doc1)
        assert_eq!(results.len(), 3);
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"d"));
        assert!(!ids.contains(&"c")); // doc3 not in filter
    }

    #[tokio::test]
    async fn test_query_filtered_all_three_fields_combined() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "match_all".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"document_id": "doc1", "tenant_id": "t1", "workspace_id": "ws1"}),
            ),
            (
                "wrong_doc".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"document_id": "doc2", "tenant_id": "t1", "workspace_id": "ws1"}),
            ),
            (
                "wrong_tenant".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"document_id": "doc1", "tenant_id": "t2", "workspace_id": "ws1"}),
            ),
            (
                "wrong_ws".to_string(),
                vec![0.7, 0.3, 0.0],
                serde_json::json!({"document_id": "doc1", "tenant_id": "t1", "workspace_id": "ws2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            document_ids: Some(vec!["doc1".to_string()]),
            tenant_id: Some("t1".to_string()),
            workspace_id: Some("ws1".to_string()),
            vector_type: None,
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "match_all");
    }

    #[tokio::test]
    async fn test_query_filtered_no_matches_returns_empty() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("nonexistent".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_query_filtered_filter_ids_empty_returns_empty() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![(
            "a".to_string(),
            vec![1.0, 0.0, 0.0],
            serde_json::json!({"tenant_id": "t1"}),
        )];
        storage.upsert(&data).await.unwrap();

        // Empty filter_ids should return nothing (no IDs match)
        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let empty_ids: Vec<String> = vec![];
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, Some(&empty_ids), Some(&mf))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_query_filtered_filter_ids_and_metadata_both_restrict() {
        // AND semantics: filter_ids restricts to {a,b}, metadata restricts to t1={a,c}
        // Intersection: {a}
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "a".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "b".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
            (
                "c".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let filter_ids = vec!["a".to_string(), "b".to_string()];
        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, Some(&filter_ids), Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn test_query_filtered_wrong_dimension_returns_error() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![(
            "a".to_string(),
            vec![1.0, 0.0, 0.0],
            serde_json::json!({"tenant_id": "t1"}),
        )];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        // Wrong dimension (2 instead of 3)
        let result = storage
            .query_filtered(&[1.0, 0.0], 10, None, Some(&mf))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_filtered_preserves_score_ordering() {
        // After filtering, results should still be ordered by similarity score (desc)
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "low".to_string(),
                vec![0.5, 0.5, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "mid".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "high".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"tenant_id": "t1"}),
            ),
            (
                "excluded".to_string(),
                vec![0.99, 0.01, 0.0],
                serde_json::json!({"tenant_id": "t2"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "high");
        assert_eq!(results[1].id, "mid");
        assert_eq!(results[2].id, "low");
        // Verify scores are monotonically decreasing
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
    }

    #[tokio::test]
    async fn test_query_filtered_source_document_id_only() {
        // Vectors with only source_document_id (no document_id) should match
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "src_only".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"source_document_id": "doc1"}),
            ),
            (
                "doc_only".to_string(),
                vec![0.9, 0.1, 0.0],
                serde_json::json!({"document_id": "doc1"}),
            ),
            (
                "both".to_string(),
                vec![0.8, 0.2, 0.0],
                serde_json::json!({"document_id": "doc2", "source_document_id": "doc1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            document_ids: Some(vec!["doc1".to_string()]),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        // All three match (src_only via source_document_id, doc_only via document_id,
        // both via source_document_id=doc1)
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_query_filtered_on_empty_storage() {
        let storage = MemoryVectorStorage::new("test", 3);

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_query_filtered_top_k_zero() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![(
            "a".to_string(),
            vec![1.0, 0.0, 0.0],
            serde_json::json!({"tenant_id": "t1"}),
        )];
        storage.upsert(&data).await.unwrap();

        let mf = MetadataFilter {
            tenant_id: Some("t1".to_string()),
            ..Default::default()
        };
        let results = storage
            .query_filtered(&[1.0, 0.0, 0.0], 0, None, Some(&mf))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    // ── Fix #208: vector_type filter — naive mode zero-result regression ─────

    /// WHY: Core regression test for issue #208.
    /// At scale, entity vectors dominate the top-k results. Without SQL-level
    /// type filtering, filter_by_type in memory would receive 0 chunks and
    /// return an empty result to the user.
    #[tokio::test]
    async fn test_query_filtered_by_vector_type_chunk_only() {
        let storage = MemoryVectorStorage::new("test", 3);
        let data = vec![
            (
                "e1".to_string(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"type": "entity", "tenant_id": "t1"}),
            ),
            (
                "e2".to_string(),
                vec![0.95, 0.1, 0.0],
                serde_json::json!({"type": "entity", "tenant_id": "t1"}),
            ),
            (
                "c1".to_string(),
                vec![0.9, 0.2, 0.0],
                serde_json::json!({"type": "chunk", "tenant_id": "t1"}),
            ),
            (
                "c2".to_string(),
                vec![0.8, 0.3, 0.0],
                serde_json::json!({"type": "chunk", "tenant_id": "t1"}),
            ),
        ];
        storage.upsert(&data).await.unwrap();

        // Without type filter — both entities and chunks returned
        let all = storage
            .query_filtered(&[1.0, 0.0, 0.0], 4, None, None)
            .await
            .unwrap();
        assert_eq!(all.len(), 4);

        // With vector_type = "chunk" — only chunks returned
        let mf =
            MetadataFilter::from_tenant_workspace_type(Some("t1".into()), None, "chunk").unwrap();
        let chunks = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(chunks.len(), 2, "must return only chunk vectors");
        assert!(chunks.iter().all(|r| r.id.starts_with('c')));

        // With vector_type = "entity" — only entities returned
        let mf_ent =
            MetadataFilter::from_tenant_workspace_type(Some("t1".into()), None, "entity").unwrap();
        let entities = storage
            .query_filtered(&[1.0, 0.0, 0.0], 10, None, Some(&mf_ent))
            .await
            .unwrap();
        assert_eq!(entities.len(), 2, "must return only entity vectors");
        assert!(entities.iter().all(|r| r.id.starts_with('e')));
    }

    /// WHY: Edge case — when ALL top-k results are entities (simulating a large
    /// graph where entity count >> chunk count), naive mode must still return
    /// the correct chunks via SQL-level filtering rather than 0 results.
    #[tokio::test]
    async fn test_query_filtered_entities_dominate_top_k_still_returns_chunks() {
        let storage = MemoryVectorStorage::new("test", 2);

        // 8 entities with very high similarity scores
        for i in 0..8 {
            storage
                .upsert(&[(
                    format!("entity_{i}"),
                    vec![1.0 - (i as f32 * 0.01), i as f32 * 0.01],
                    serde_json::json!({"type": "entity"}),
                )])
                .await
                .unwrap();
        }
        // 2 chunks with lower scores (would be excluded by top-k without type filter)
        storage
            .upsert(&[
                (
                    "chunk_a".to_string(),
                    vec![0.85, 0.15],
                    serde_json::json!({"type": "chunk"}),
                ),
                (
                    "chunk_b".to_string(),
                    vec![0.80, 0.20],
                    serde_json::json!({"type": "chunk"}),
                ),
            ])
            .await
            .unwrap();

        // Without type filter, top-5 would be all entities → chunks never returned
        let top5 = storage
            .query_filtered(&[1.0, 0.0], 5, None, None)
            .await
            .unwrap();
        let chunk_count_without_filter = top5.iter().filter(|r| r.id.starts_with("chunk")).count();
        assert_eq!(chunk_count_without_filter, 0, "top-5 dominated by entities");

        // With vector_type = "chunk", chunks ARE returned regardless of score rank
        let mf = MetadataFilter::from_tenant_workspace_type(None, None, "chunk").unwrap();
        let chunk_results = storage
            .query_filtered(&[1.0, 0.0], 5, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(chunk_results.len(), 2, "type filter restores chunk results");
        assert!(chunk_results.iter().all(|r| r.id.starts_with("chunk")));
    }

    /// WHY: Records without a "type" field in metadata must be EXCLUDED when
    /// a vector_type filter is active. This prevents old/legacy vectors
    /// (ingested before the type tag was added) from polluting filtered results.
    #[tokio::test]
    async fn test_query_filtered_vector_type_excludes_untyped_records() {
        let storage = MemoryVectorStorage::new("test", 2);
        storage
            .upsert(&[
                (
                    "typed_chunk".to_string(),
                    vec![1.0, 0.0],
                    serde_json::json!({"type": "chunk"}),
                ),
                (
                    "no_type".to_string(),
                    vec![0.99, 0.01],
                    serde_json::json!({"description": "no type field"}),
                ),
            ])
            .await
            .unwrap();

        let mf = MetadataFilter::from_tenant_workspace_type(None, None, "chunk").unwrap();
        let results = storage
            .query_filtered(&[1.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "typed_chunk");
    }

    /// WHY: vector_type filter must combine correctly with tenant + workspace
    /// filters. An entity vector with correct type but wrong tenant must be excluded.
    #[tokio::test]
    async fn test_query_filtered_vector_type_combines_with_tenant_workspace() {
        let storage = MemoryVectorStorage::new("test", 2);
        storage
            .upsert(&[
                (
                    "chunk_t1_ws1".to_string(),
                    vec![1.0, 0.0],
                    serde_json::json!({"type": "chunk", "tenant_id": "t1", "workspace_id": "ws1"}),
                ),
                (
                    "chunk_t2_ws1".to_string(),
                    vec![0.99, 0.01],
                    serde_json::json!({"type": "chunk", "tenant_id": "t2", "workspace_id": "ws1"}),
                ),
                (
                    "entity_t1_ws1".to_string(),
                    vec![0.98, 0.02],
                    serde_json::json!({"type": "entity", "tenant_id": "t1", "workspace_id": "ws1"}),
                ),
            ])
            .await
            .unwrap();

        let mf = MetadataFilter::from_tenant_workspace_type(
            Some("t1".into()),
            Some("ws1".into()),
            "chunk",
        )
        .unwrap();
        let results = storage
            .query_filtered(&[1.0, 0.0], 10, None, Some(&mf))
            .await
            .unwrap();
        // Only chunk_t1_ws1 satisfies all three predicates
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "chunk_t1_ws1");
    }

    /// WHY: top_k must apply AFTER the type filter, not to the full unfiltered
    /// collection. If top_k=1 and type="chunk", we must get 1 chunk — not the
    /// highest-scoring item across all types then filtered.
    #[tokio::test]
    async fn test_query_filtered_top_k_applied_after_type_filter() {
        let storage = MemoryVectorStorage::new("test", 2);
        storage
            .upsert(&[
                (
                    "entity_hi".to_string(),
                    vec![1.0, 0.0],
                    serde_json::json!({"type": "entity"}),
                ),
                (
                    "chunk_1".to_string(),
                    vec![0.9, 0.1],
                    serde_json::json!({"type": "chunk"}),
                ),
                (
                    "chunk_2".to_string(),
                    vec![0.8, 0.2],
                    serde_json::json!({"type": "chunk"}),
                ),
            ])
            .await
            .unwrap();

        let mf = MetadataFilter::from_tenant_workspace_type(None, None, "chunk").unwrap();
        let results = storage
            .query_filtered(&[1.0, 0.0], 1, None, Some(&mf))
            .await
            .unwrap();
        // top_k=1 among chunks → should be chunk_1 (highest chunk score)
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "chunk_1");
    }
}
