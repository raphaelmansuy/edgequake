//! Response DTOs for workspace management API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Tenant / Workspace Responses ──────────────────────────────────────────

/// Tenant response DTO.
///
/// Includes default model configuration (SPEC-032) for new workspaces.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResponse {
    /// Tenant ID.
    pub id: Uuid,
    /// Tenant name.
    pub name: String,
    /// URL-friendly slug.
    pub slug: String,
    /// Plan type.
    pub plan: String,
    /// Whether the tenant is active.
    pub is_active: bool,
    /// Maximum workspaces allowed.
    pub max_workspaces: usize,

    // === Default LLM Configuration (SPEC-032) ===
    /// Default LLM model for new workspaces.
    pub default_llm_model: String,
    /// Default LLM provider for new workspaces.
    pub default_llm_provider: String,
    /// Fully qualified default LLM model ID (provider/model format).
    pub default_llm_full_id: String,

    // === Default Embedding Configuration (SPEC-032) ===
    /// Default embedding model for new workspaces.
    pub default_embedding_model: String,
    /// Default embedding provider for new workspaces.
    pub default_embedding_provider: String,
    /// Default embedding dimension for new workspaces.
    pub default_embedding_dimension: usize,
    /// Fully qualified default embedding model ID (provider/model format).
    pub default_embedding_full_id: String,

    // === Default Vision LLM Configuration (SPEC-041) ===
    /// Default Vision LLM model for PDF-to-Markdown extraction.
    /// None if not configured (workspaces use upload-time defaults).
    pub default_vision_llm_model: Option<String>,
    /// Default Vision LLM provider for PDF-to-Markdown extraction.
    /// None if not configured.
    pub default_vision_llm_provider: Option<String>,

    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Workspace response DTO.
///
/// Includes full model configuration (SPEC-032) for transparency.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceResponse {
    /// Workspace ID.
    pub id: Uuid,
    /// Parent tenant ID.
    pub tenant_id: Uuid,
    /// Workspace name.
    pub name: String,
    /// URL-friendly slug.
    pub slug: String,
    /// Description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: bool,
    /// Maximum documents allowed.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model for knowledge graph generation and summarization.
    pub llm_model: String,
    /// LLM provider (openai, ollama, lmstudio).
    pub llm_provider: String,
    /// Fully qualified LLM model ID (provider/model format).
    pub llm_full_id: String,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model used for this workspace.
    pub embedding_model: String,
    /// Embedding provider (openai, ollama, lmstudio).
    pub embedding_provider: String,
    /// Embedding vector dimension.
    pub embedding_dimension: usize,
    /// Fully qualified embedding model ID (provider/model format).
    pub embedding_full_id: String,

    // === Vision LLM Configuration (SPEC-040) ===
    /// Vision LLM provider for PDF → Markdown extraction (None if not configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_provider: Option<String>,
    /// Vision LLM model for PDF page image extraction (None if not configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_model: Option<String>,
    /// Default PDF parser backend for this workspace (None means server default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_parser_backend: Option<String>,

    // === Entity Type Configuration (SPEC-085) ===
    /// Custom entity types configured for this workspace.
    /// None means the workspace uses server default_entity_types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// When true, unknown types are remapped to OTHER/CONCEPT (default true).
    pub entity_types_strict: bool,

    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

// ── List Responses ────────────────────────────────────────────────────────

/// List response with pagination info.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantListResponse {
    /// Items in this page.
    pub items: Vec<TenantResponse>,
    /// Total count.
    pub total: usize,
    /// Current offset.
    pub offset: usize,
    /// Page size limit.
    pub limit: usize,
}

/// List response with pagination info.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceListResponse {
    /// Items in this page.
    pub items: Vec<WorkspaceResponse>,
    /// Total count.
    pub total: usize,
    /// Current offset.
    pub offset: usize,
    /// Page size limit.
    pub limit: usize,
}

// ── Pagination and Stats ──────────────────────────────────────────────────

/// Pagination query params.
#[derive(Debug, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationParams {
    /// Offset (default 0).
    #[serde(default)]
    pub offset: usize,
    /// Limit (default 20, max 100).
    #[serde(default = "workspaces_default_limit")]
    pub limit: usize,
}

/// Default limit for workspace pagination.
pub fn workspaces_default_limit() -> usize {
    20
}

/// Workspace statistics response.
///
/// WHY embedding_count: Mission requirement to track embeddings per workspace.
/// WHY entity_type_count: Dashboard EntityTypes KPI was very slow because the
/// frontend fetched ALL graph nodes just to count unique types. This field
/// delivers the count from a single Cypher aggregate query (<1ms vs 2-5s).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkspaceStatsResponse {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Number of documents.
    pub document_count: usize,
    /// Number of entities (graph nodes).
    pub entity_count: usize,
    /// Number of relationships (graph edges).
    pub relationship_count: usize,
    /// Number of distinct entity types (e.g., PERSON, ORGANIZATION, …).
    pub entity_type_count: usize,
    /// Number of chunks (text segments).
    pub chunk_count: usize,
    /// Number of embeddings (vector representations).
    pub embedding_count: usize,
    /// Storage used in bytes.
    pub storage_bytes: u64,
}

/// Single metrics snapshot for historical data.
///
/// OODA-22: Individual snapshot in metrics history response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MetricsSnapshotDTO {
    /// Unique snapshot ID.
    pub id: Uuid,
    /// When the snapshot was recorded.
    pub recorded_at: String,
    /// What triggered the recording (event, scheduled, manual).
    pub trigger_type: String,
    /// Number of documents.
    pub document_count: i64,
    /// Number of chunks.
    pub chunk_count: i64,
    /// Number of entities.
    pub entity_count: i64,
    /// Number of relationships.
    pub relationship_count: i64,
    /// Number of embeddings.
    pub embedding_count: i64,
    /// Storage bytes.
    pub storage_bytes: i64,
}

/// Metrics history response with pagination.
///
/// OODA-22: Response for GET /workspaces/{id}/metrics-history endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct MetricsHistoryResponse {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// List of metrics snapshots (newest first).
    pub snapshots: Vec<MetricsSnapshotDTO>,
    /// Number of snapshots returned.
    pub count: usize,
    /// Offset used for pagination.
    pub offset: usize,
    /// Limit used for pagination.
    pub limit: usize,
}
