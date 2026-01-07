//! Vector storage trait.

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
}
