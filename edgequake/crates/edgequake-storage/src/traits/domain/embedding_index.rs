//! SPEC-091 EmbeddingIndex port — typed vector search behind adapters.

use async_trait::async_trait;

use crate::error::StorageError;

use super::types::{
    EmbeddingCapabilities, EmbeddingRow, ModelId, ScoredChunk, UpsertReport, VectorQuery,
    WorkspaceId,
};

#[async_trait]
pub trait EmbeddingIndex: Send + Sync {
    fn capabilities(&self) -> EmbeddingCapabilities;

    /// The pgvector applier writes typed Postgres tables itself.
    /// An index that stores somewhere else must take the batch.
    fn uses_external_upsert(&self) -> bool {
        false
    }

    async fn upsert_batch(
        &self,
        model: ModelId,
        rows: &[EmbeddingRow],
    ) -> Result<UpsertReport, StorageError>;

    async fn search(&self, req: &VectorQuery) -> Result<Vec<ScoredChunk>, StorageError>;

    async fn delete_for_workspace(&self, workspace: WorkspaceId) -> Result<u64, StorageError>;
}
