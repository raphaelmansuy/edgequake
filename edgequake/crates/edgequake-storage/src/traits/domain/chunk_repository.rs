//! SPEC-091 ChunkRepository port — batch-first relational chunk authority.

use async_trait::async_trait;

use crate::error::StorageError;

use super::types::{
    Chunk, ChunkCursor, ChunkId, ChunkText, DocumentId, InsertReport, Page, UnitOfWork,
};

/// Batch-first chunk persistence (LAW-D7: no per-row round trips in port API).
#[async_trait]
pub trait ChunkRepository: Send + Sync {
    async fn insert_batch(
        &self,
        tx: &mut UnitOfWork,
        chunks: &[Chunk],
    ) -> Result<InsertReport, StorageError>;

    async fn load_texts(&self, ids: &[ChunkId]) -> Result<Vec<ChunkText>, StorageError>;

    /// W1 read cutover: every chunk of one document, ordered by `chunk_index`.
    async fn load_for_document(&self, document_id: DocumentId) -> Result<Vec<Chunk>, StorageError>;

    /// W1 read cutover: single chunk by its unique `(document_id, chunk_index)`.
    async fn load_one(
        &self,
        document_id: DocumentId,
        chunk_index: i32,
    ) -> Result<Option<Chunk>, StorageError>;

    /// W1 read cutover: chunk count for one document (replaces KV prefix scans).
    async fn count_for_document(&self, document_id: DocumentId) -> Result<u64, StorageError>;

    async fn scan_from(
        &self,
        cursor: Option<ChunkCursor>,
        limit: u32,
    ) -> Result<Page<Chunk>, StorageError>;

    async fn delete_for_document(
        &self,
        tx: &mut UnitOfWork,
        document_id: DocumentId,
    ) -> Result<u64, StorageError>;

    /// W4 serving lifecycle: set `chunk_serving_state.state` for every chunk of
    /// a document.
    async fn set_serving_state(
        &self,
        document_id: DocumentId,
        state: &str,
    ) -> Result<u64, StorageError>;
}
