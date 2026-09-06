//! SPEC-091 domain types — storage-agnostic placeholders for port boundaries.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Typed chunk identifier (relational authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub Uuid);

impl ChunkId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<Uuid> for ChunkId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub Uuid);

/// Authoritative chunk row for relational insert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub document_id: DocumentId,
    pub tenant_id: Option<TenantId>,
    pub workspace_id: Option<WorkspaceId>,
    pub chunk_index: i32,
    pub content: String,
    pub start_offset: Option<i32>,
    pub end_offset: Option<i32>,
    pub token_count: Option<i32>,
    pub metadata: serde_json::Value,
    /// SPEC-135: PDF page start (1-indexed). None when unmarked.
    #[serde(default)]
    pub page_start: Option<i32>,
    /// SPEC-135: PDF page end (`≥ page_start`).
    #[serde(default)]
    pub page_end: Option<i32>,
}

/// Text payload returned by load_texts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkText {
    pub id: ChunkId,
    pub content: String,
}

/// Keyset pagination cursor for scan_from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkCursor {
    pub document_id: DocumentId,
    pub chunk_index: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsertReport {
    pub inserted: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpsertReport {
    pub upserted: u64,
    /// SPEC-120: rows skipped because `(workspace_id, legacy_vector_id)` was
    /// already owned by a different FK (absorbable 23505).
    #[serde(default)]
    pub absorbed_legacy_collisions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<ChunkCursor>,
}

/// Placeholder transaction handle — adapters map to sqlx TX or in-memory locks.
#[derive(Debug, Default)]
pub struct UnitOfWork {
    pub label: Option<String>,
}

/// Vector query request (minimal stub for EmbeddingIndex port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQuery {
    pub model_id: ModelId,
    pub workspace_id: Option<WorkspaceId>,
    pub embedding: Vec<f32>,
    pub limit: u32,
    /// SPEC-146: document allow-set for ANN pre-filter.
    /// `None` = no document scope (flag off / unscoped).
    /// `Some([])` = fail-closed empty (ABAC on, no authorized docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_document_ids: Option<Vec<uuid::Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredChunk {
    pub chunk_id: ChunkId,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRow {
    pub chunk_id: ChunkId,
    pub workspace_id: WorkspaceId,
    pub embedding: Vec<f32>,
    pub dimensions: i32,
}

/// IW2 typed fleet embedding row (entity / relationship / report).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetEmbeddingRow {
    pub workspace_id: WorkspaceId,
    pub embedding: Vec<f32>,
    pub dimensions: i32,
    /// Entity UUID, relationship UUID, or legacy report TEXT id.
    pub key: FleetEmbeddingKey,
    /// SPEC-111: source `eq_*_vectors.id` for migration 131 provenance (optional).
    #[serde(default)]
    pub legacy_vector_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FleetEmbeddingKey {
    Entity(uuid::Uuid),
    Relationship(uuid::Uuid),
    Report(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredFleet {
    pub legacy_id: String,
    pub score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingCapabilities {
    pub metric: &'static str,
    pub supports_filters: bool,
    pub supports_rerank: bool,
}
