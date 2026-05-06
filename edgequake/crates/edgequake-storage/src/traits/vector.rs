//! Vector storage trait for similarity search.
//!
//! # Implements
//!
//! - **FEAT0201**: Vector Similarity Search
//!
//! # Enforces
//!
//! - **BR0201**: Namespace-based tenant isolation
//! - **BR0010**: Embedding dimension validated on insert
//!
//! # WHY: Separate Vector Storage
//!
//! Vector similarity search is specialized:
//! - Requires optimized index structures (HNSW, IVF)
//! - Benefits from GPU acceleration
//! - Different scaling characteristics than graph/KV
//!
//! Abstracting as a trait allows using:
//! - pgvector (PostgreSQL extension)
//! - Pinecone, Weaviate, Qdrant (managed services)
//! - In-memory brute-force (testing)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Vector similarity search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// Record identifier
    pub id: String,
    /// Similarity score (higher is more similar)
    pub score: f32,
    /// Associated metadata
    pub metadata: serde_json::Value,
}

/// Metadata-based filter for vector queries (SPEC-007 Tier 2+).
///
/// All fields are optional; only non-None fields participate in AND-combined filtering.
/// Pushes filtering to the SQL layer (JSONB WHERE or column WHERE) to avoid
/// retrieving and discarding irrelevant vectors in application code.
///
/// @implements SPEC-007 R-T2-01
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataFilter {
    /// Filter by document ID(s). Matches JSONB key `document_id` OR `source_document_id`.
    pub document_ids: Option<Vec<String>>,
    /// Filter by tenant ID.
    pub tenant_id: Option<String>,
    /// Filter by workspace ID.
    pub workspace_id: Option<String>,
    /// Filter by vector type (e.g. "chunk", "entity", "relationship").
    ///
    /// WHY: At scale (60k+ entities, 10k+ chunks), the top-k results from a workspace
    /// vector table are dominated by entity vectors if no type filter is applied.
    /// Pushing type filtering to the SQL layer ensures the LIMIT clause operates on
    /// the correct vector type, preventing naive mode from returning 0 chunks.
    pub vector_type: Option<String>,
}

impl MetadataFilter {
    /// Returns true when no filter fields are set.
    pub fn is_empty(&self) -> bool {
        self.document_ids.is_none()
            && self.tenant_id.is_none()
            && self.workspace_id.is_none()
            && self.vector_type.is_none()
    }

    /// Build a filter from optional tenant and workspace IDs.
    pub fn from_tenant_workspace(
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Option<Self> {
        if tenant_id.is_none() && workspace_id.is_none() {
            return None;
        }
        Some(Self {
            document_ids: None,
            tenant_id,
            workspace_id,
            vector_type: None,
        })
    }

    /// Build a filter with tenant, workspace, and vector type.
    ///
    /// WHY: Naive mode must filter by type=chunk at the SQL level to avoid returning
    /// entity/relationship vectors when the top-k results are entity-dominated.
    pub fn from_tenant_workspace_type(
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_type: impl Into<String>,
    ) -> Option<Self> {
        Some(Self {
            document_ids: None,
            tenant_id,
            workspace_id,
            vector_type: Some(vector_type.into()),
        })
    }
}

/// Vector storage interface for similarity search.
///
/// Provides storage and retrieval of vector embeddings with
/// support for similarity search operations.
///
/// # Implementations
///
/// - `MemoryVectorStorage` - In-memory brute-force search (testing)
/// - `PgVectorStorage` - PostgreSQL with pgvector extension
/// - `SurrealDBVectorStorage` - SurrealDB native vector support
#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// Get the storage namespace.
    fn namespace(&self) -> &str;

    /// Get the expected embedding dimension.
    fn dimension(&self) -> usize;

    /// Initialize the vector storage.
    ///
    /// Creates necessary indices and tables.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;

    /// Perform similarity search.
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - The query vector
    /// * `top_k` - Maximum number of results to return
    /// * `filter_ids` - Optional list of IDs to restrict search to
    ///
    /// # Returns
    ///
    /// Vector of search results ordered by similarity (highest first).
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>>;

    /// Insert or update vectors with metadata.
    ///
    /// # Arguments
    ///
    /// * `data` - Vector of (id, embedding, metadata) tuples
    async fn upsert(&self, data: &[(String, Vec<f32>, serde_json::Value)]) -> Result<()>;

    /// Delete vectors by IDs.
    async fn delete(&self, ids: &[String]) -> Result<()>;

    /// Delete all vectors associated with an entity.
    ///
    /// This is used when deleting an entity to clean up its embeddings.
    async fn delete_entity(&self, entity_name: &str) -> Result<()>;

    /// Delete all relationship vectors involving an entity.
    ///
    /// Used when cascading entity deletion.
    async fn delete_entity_relations(&self, entity_name: &str) -> Result<()>;

    /// Get a single vector by ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>>;

    /// Get multiple vectors by IDs.
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>>;

    /// Check if storage is empty.
    async fn is_empty(&self) -> Result<bool>;

    /// Get count of stored vectors.
    async fn count(&self) -> Result<usize>;

    /// Clear all vectors.
    async fn clear(&self) -> Result<()>;

    /// Clear vectors for a specific workspace.
    ///
    /// This is used when rebuilding embeddings for a single workspace
    /// without affecting other workspaces.
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - The UUID of the workspace to clear vectors for
    ///
    /// # Returns
    ///
    /// Number of vectors deleted.
    ///
    /// # Default Implementation
    ///
    /// Returns 0 by default. Implementations should override this for
    /// workspace-scoped clearing.
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        // Default implementation does nothing - clear() clears all
        // Implementations should override this for workspace-scoped clearing
        let _ = workspace_id;
        Ok(0)
    }

    /// Query with metadata pre-filter (SPEC-007 Tier 2+).
    ///
    /// Pushes tenant/workspace/document filters to the storage layer (SQL WHERE)
    /// instead of post-filtering in application code.
    ///
    /// Default implementation ignores `metadata_filter` and delegates to `query()`.
    /// Backends that support SQL-level filtering override this for better performance.
    ///
    /// @implements SPEC-007 R-T2-01
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        let _ = metadata_filter;
        self.query(query_embedding, top_k, filter_ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_filter_is_empty() {
        let f = MetadataFilter::default();
        assert!(f.is_empty());

        let f = MetadataFilter {
            tenant_id: Some("t1".into()),
            ..Default::default()
        };
        assert!(!f.is_empty());
    }

    #[test]
    fn test_metadata_filter_from_tenant_workspace_both_none() {
        assert!(MetadataFilter::from_tenant_workspace(None, None).is_none());
    }

    #[test]
    fn test_metadata_filter_from_tenant_workspace_tenant_only() {
        let mf = MetadataFilter::from_tenant_workspace(Some("t1".into()), None).unwrap();
        assert_eq!(mf.tenant_id.as_deref(), Some("t1"));
        assert!(mf.workspace_id.is_none());
        assert!(mf.document_ids.is_none());
    }

    #[test]
    fn test_metadata_filter_from_tenant_workspace_both() {
        let mf =
            MetadataFilter::from_tenant_workspace(Some("t1".into()), Some("ws1".into())).unwrap();
        assert_eq!(mf.tenant_id.as_deref(), Some("t1"));
        assert_eq!(mf.workspace_id.as_deref(), Some("ws1"));
    }

    #[test]
    fn test_metadata_filter_serialization_roundtrip() {
        let mf = MetadataFilter {
            document_ids: Some(vec!["doc1".into(), "doc2".into()]),
            tenant_id: Some("t1".into()),
            workspace_id: Some("ws1".into()),
            vector_type: None,
        };
        let json = serde_json::to_string(&mf).unwrap();
        let mf2: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(mf2.tenant_id, mf.tenant_id);
        assert_eq!(mf2.workspace_id, mf.workspace_id);
        assert_eq!(mf2.document_ids, mf.document_ids);
    }
}
