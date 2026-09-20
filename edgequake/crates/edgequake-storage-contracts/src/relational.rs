//! Narrow relational read and atomic command ports.

use crate::error::AccessResult;
use crate::ids::DocumentId;
use crate::scope::AccessScope;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SHA-256 digest bytes used by canonical command encodings.
pub type Digest = [u8; 32];

/// One immutable prepared row carried by an ingestion batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedRecord {
    pub id: Uuid,
    pub revision: u64,
    pub digest: Digest,
    pub payload: Vec<u8>,
}

/// A bounded, fully validated ingestion command ready for atomic persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedIngestionBatch {
    pub scope: AccessScope,
    pub document_id: DocumentId,
    pub ingest_generation: u64,
    pub batch_ordinal: u64,
    pub expected_revision: Option<u64>,
    pub idempotency_key: String,
    pub schema_version: u32,
    pub canonical_digest: Digest,
    pub chunks: Vec<PreparedRecord>,
    pub facts: Vec<PreparedRecord>,
    pub contributions: Vec<PreparedRecord>,
    pub embeddings: Vec<PreparedRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedRevision {
    pub id: Uuid,
    pub revision: u64,
}

/// Durable authority receipt, independent of projection completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitReceipt {
    pub request_key: String,
    pub command_digest: Digest,
    pub document_generation: u64,
    pub committed: Vec<CommittedRevision>,
    pub manifest_id: Uuid,
    pub durable_commit_token: String,
}

/// Compare-and-tombstone lifecycle command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteDocument {
    pub scope: AccessScope,
    pub document_id: DocumentId,
    pub expected_revision: u64,
    pub idempotency_key: String,
    pub command_digest: Digest,
}

/// Logical deletion receipt; physical provider cleanup may still be pending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteReceipt {
    pub scope: AccessScope,
    pub document_id: DocumentId,
    pub tombstone_revision: u64,
    pub cleanup_manifest_id: Uuid,
    pub target_binding_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentView {
    pub scope: AccessScope,
    pub document_id: DocumentId,
    pub revision: u64,
    pub digest: Digest,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentPageRequest {
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[async_trait]
pub trait DocumentReader: Send + Sync {
    async fn get_many(
        &self,
        scope: &AccessScope,
        ids: &[DocumentId],
    ) -> AccessResult<Vec<Option<DocumentView>>>;

    async fn list(
        &self,
        scope: &AccessScope,
        request: &DocumentPageRequest,
    ) -> AccessResult<CursorPage<DocumentView>>;
}

#[async_trait]
pub trait IngestionCommitter: Send + Sync {
    async fn commit_batch(&self, command: &PreparedIngestionBatch) -> AccessResult<CommitReceipt>;
}

#[async_trait]
pub trait LifecycleCommitter: Send + Sync {
    async fn tombstone_document(&self, command: &DeleteDocument) -> AccessResult<DeleteReceipt>;
}
