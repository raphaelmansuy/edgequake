//! DTOs for rebuild / reprocess operations on workspaces.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Rebuild Embeddings (SPEC-032) ─────────────────────────────────────────

/// Request to rebuild workspace embeddings with a new model.
///
/// This operation:
/// 1. Updates the workspace embedding configuration
/// 2. Clears all existing vector embeddings
/// 3. Triggers re-embedding of all documents (async background job)
///
/// ## WARNING
///
/// This is a destructive operation that will delete all existing embeddings.
/// Queries will return no results until re-embedding is complete.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RebuildEmbeddingsRequest {
    /// New embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// If not provided, uses the current workspace model (just clears and re-embeds).
    pub embedding_model: Option<String>,

    /// New embedding provider ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected from embedding_model or keeps current.
    pub embedding_provider: Option<String>,

    /// New embedding dimension.
    /// If not provided, auto-detected from embedding_model or keeps current.
    pub embedding_dimension: Option<usize>,

    /// Whether to force rebuild even if embedding config is unchanged.
    /// Useful for refreshing embeddings after model updates.
    #[serde(default)]
    pub force: bool,
}

/// Response from rebuild embeddings operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct RebuildEmbeddingsResponse {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Status of the operation ("started", "in_progress", "completed", "failed").
    pub status: String,
    /// Number of documents to be re-embedded.
    pub documents_to_process: usize,
    /// Total number of chunks across all documents to be re-embedded.
    /// This provides a more accurate estimate of processing time than document count.
    pub chunks_to_process: usize,
    /// Number of vectors cleared.
    pub vectors_cleared: usize,
    /// New embedding model (after update).
    pub embedding_model: String,
    /// New embedding provider (after update).
    pub embedding_provider: String,
    /// New embedding dimension (after update).
    pub embedding_dimension: usize,
    /// New embedding model's context length (max input tokens).
    /// REQ-25: Chunk compatibility validation.
    pub model_context_length: usize,
    /// Estimated time to complete (seconds).
    pub estimated_time_seconds: Option<u64>,
    /// Background job ID for tracking (if async).
    pub job_id: Option<String>,
    /// Level 4 v2 migration hint (additive — SPEC-027 phase 21).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_migration: Option<crate::services::job_registry::V2MigrationHint>,
    /// Warning message if chunk size exceeds model context length.
    /// REQ-25: Critical invariant - chunks must fit model's input limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_warning: Option<String>,
}

// ── Reprocess All Documents (SPEC-032 Focus Area 5) ───────────────────────

/// Request to reprocess all documents in a workspace.
///
/// This operation queues all documents for re-embedding, typically used after
/// a rebuild-embeddings operation to regenerate vector embeddings.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReprocessAllRequest {
    /// Whether to include successfully processed documents.
    /// If false, only pending/failed documents are reprocessed.
    #[serde(default = "default_include_completed")]
    pub include_completed: bool,

    /// Maximum number of documents to process.
    /// Default: 1000.
    #[serde(default = "default_max_reprocess")]
    pub max_documents: usize,
}

fn default_include_completed() -> bool {
    true
}

fn default_max_reprocess() -> usize {
    1000
}

/// Response from reprocess all operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReprocessAllResponse {
    /// Track ID for monitoring progress.
    pub track_id: String,
    /// Level 4 v2 migration hint (additive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_migration: Option<crate::services::job_registry::V2MigrationHint>,
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Status of the operation.
    pub status: String,
    /// Total documents found.
    pub documents_found: usize,
    /// Documents queued for processing.
    pub documents_queued: usize,
    /// Documents skipped (already processing or other reasons).
    pub documents_skipped: usize,
    /// Estimated processing time in seconds.
    pub estimated_time_seconds: Option<u64>,
}

// ── Rebuild Knowledge Graph (LLM Model Change) ───────────────────────────

/// Request to rebuild knowledge graph for a workspace.
///
/// Used when the LLM (extraction) model changes. This operation:
/// 1. Clears all entities and relationships from the graph
/// 2. Clears all vector embeddings
/// 3. Triggers reprocessing of all documents
///
/// ## WARNING
///
/// This is a destructive operation that will delete all extracted knowledge.
/// The workspace will be empty until reprocessing is complete.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RebuildKnowledgeGraphRequest {
    /// New LLM model name (e.g., "gpt-4o-mini", "gemma3:12b").
    /// If not provided, uses the current workspace model.
    pub llm_model: Option<String>,

    /// New LLM provider ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected or keeps current.
    pub llm_provider: Option<String>,

    /// Whether to force rebuild even if LLM config is unchanged.
    /// Useful for refreshing extractions after model updates.
    #[serde(default)]
    pub force: bool,

    /// Whether to also rebuild embeddings (trigger vector rebuild).
    /// Default: true (recommended, as chunks may change).
    #[serde(default = "default_rebuild_embeddings")]
    pub rebuild_embeddings: bool,

    /// Maximum documents to reprocess (for large workspaces).
    /// Default: 10000.
    #[serde(default = "default_max_reprocess_kg")]
    pub max_documents: usize,
}

fn default_rebuild_embeddings() -> bool {
    true
}

fn default_max_reprocess_kg() -> usize {
    10000
}

/// Response from rebuild knowledge graph operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct RebuildKnowledgeGraphResponse {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Status of the operation.
    pub status: String,
    /// Number of nodes (entities) cleared from the graph.
    pub nodes_cleared: usize,
    /// Number of edges (relationships) cleared from the graph.
    pub edges_cleared: usize,
    /// Number of vectors cleared (if rebuild_embeddings was true).
    pub vectors_cleared: usize,
    /// Number of documents to be reprocessed.
    pub documents_to_process: usize,
    /// Total number of chunks across all documents to be reprocessed.
    pub chunks_to_process: usize,
    /// New LLM model (after update).
    pub llm_model: String,
    /// New LLM provider (after update).
    pub llm_provider: String,
    /// Estimated time to complete (seconds).
    pub estimated_time_seconds: Option<u64>,
    /// Track ID for monitoring progress.
    pub track_id: Option<String>,
    /// Level 4 v2 migration hint (additive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_migration: Option<crate::services::job_registry::V2MigrationHint>,
}
