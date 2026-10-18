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

    /// Whether projection upserts must be delivered to this index.
    ///
    /// Built-in pgvector indexes return false: the applier opens one typed
    /// table per payload `model_id`. An injected index returns true so those
    /// rows reach the provider the query path actually searches.
    fn receive_projection_upserts(&self) -> bool {
        true
    }

    async fn upsert_batch(
        &self,
        model: ModelId,
        rows: &[EmbeddingRow],
    ) -> Result<UpsertReport, StorageError>;

    async fn search(&self, req: &VectorQuery) -> Result<Vec<ScoredChunk>, StorageError>;

    async fn delete_for_workspace(&self, workspace: WorkspaceId) -> Result<u64, StorageError>;
}
