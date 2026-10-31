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

    /// SPEC-109: default reasoning effort seed for workspaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reasoning_effort: Option<String>,

    /// SPEC-123: tenant default PDF parser
    /// (`vision` | `edgeparse` | `edgeparse-ocr` | `auto`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<crate::handlers::PdfParserBackendName>, example = "edgeparse-ocr")]
    pub pdf_parser_backend: Option<String>,

    /// Tenant brand color (`#rrggbb`). `null` means the EdgeQuake default accent.
    pub primary_color: Option<String>,

    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

impl TenantResponse {
    /// The one tenant → wire mapping used by every tenant endpoint.
    ///
    /// WHY: five handlers used to rebuild this struct by hand, so a new tenant field
    /// had to be remembered in each copy. Mapping in one place keeps them in lockstep.
    pub fn from_tenant(tenant: &edgequake_core::Tenant) -> Self {
        Self {
            id: tenant.tenant_id,
            name: tenant.name.clone(),
            slug: tenant.slug.clone(),
            plan: format!("{}", tenant.plan),
            is_active: tenant.is_active,
            max_workspaces: tenant.max_workspaces,
            default_llm_model: tenant.default_llm_model.clone(),
            default_llm_provider: tenant.default_llm_provider.clone(),
            default_llm_full_id: format!(
                "{}/{}",
                tenant.default_llm_provider, tenant.default_llm_model
            ),
            default_embedding_model: tenant.default_embedding_model.clone(),
            default_embedding_provider: tenant.default_embedding_provider.clone(),
            default_embedding_dimension: tenant.default_embedding_dimension,
            default_embedding_full_id: format!(
                "{}/{}",
                tenant.default_embedding_provider, tenant.default_embedding_model
            ),
            default_vision_llm_model: tenant.default_vision_llm_model.clone(),
            default_vision_llm_provider: tenant.default_vision_llm_provider.clone(),
            default_reasoning_effort: tenant.default_reasoning_effort.clone(),
            pdf_parser_backend: tenant.pdf_parser_backend.map(|b| b.as_str().to_string()),
            primary_color: tenant.primary_color.clone(),
            created_at: tenant.created_at.to_rfc3339(),
            updated_at: tenant.updated_at.to_rfc3339(),
        }
    }
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

    /// SPEC-123: effective LLM after Request>Workspace>Tenant>Env (LAW-123-8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_llm_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_llm_model: Option<String>,
    /// Provenance: `workspace` | `tenant` | `env` | `default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_resolution_source: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model used for this workspace.
    pub embedding_model: String,
    /// Embedding provider (openai, ollama, lmstudio).
    pub embedding_provider: String,
    /// Embedding vector dimension.
    pub embedding_dimension: usize,
    /// Fully qualified embedding model ID (provider/model format).
    pub embedding_full_id: String,

    /// SPEC-123: effective embedding stack with provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_embedding_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_embedding_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_embedding_dimension: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_resolution_source: Option<String>,
    /// Set on a workspace update when the resolved embedding model, provider, or dimension
    /// changed. Stored vectors came from the previous embedder and need a rebuild (SPEC-178).
    #[serde(default)]
    pub embedding_rebuild_required: bool,
    /// Resolved embedding dimension before the update; set only when a rebuild is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_embedding_dimension: Option<usize>,

    // === Vision LLM Configuration (SPEC-040) ===
    /// Vision LLM provider for PDF → Markdown extraction (None if not configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_provider: Option<String>,
    /// Vision LLM model for PDF page image extraction (None if not configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_model: Option<String>,

    /// SPEC-123: effective vision LLM (VLM — not an embedding model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_vision_llm_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_vision_llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_resolution_source: Option<String>,

    /// Default PDF parser backend for this workspace
    /// (`vision` | `edgeparse` | `edgeparse-ocr` | `auto`; None means server default).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<crate::handlers::PdfParserBackendName>, example = "edgeparse-ocr")]
    pub pdf_parser_backend: Option<String>,

    // === Entity Type Configuration (SPEC-085) ===
    /// Custom entity types configured for this workspace.
    /// None means the workspace uses server default_entity_types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// When true, unknown types are remapped to OTHER/CONCEPT (default true).
    pub entity_types_strict: bool,

    /// Configured extraction language override (SPEC-096).
    /// `null` / omitted means inherit `EDGEQUAKE_EXTRACTION_LANGUAGE` or English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_language: Option<String>,

    /// SPEC-116 chunking mode (`adaptive` | `fixed`). Absent = inherit fleet env.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_token_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_overlap_token_size: Option<u32>,

    /// SPEC-117: absent = inherit fleet env; present when custom ints stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_budget_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_max_entities: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_max_records: Option<u32>,

    /// SPEC-160: workspace extraction mode, effective mode, and decision settings.
    #[serde(flatten)]
    pub extraction: super::ExtractionModeView,

    /// SPEC-180: retrieval profile, modalities, text-search config and capabilities.
    #[serde(flatten)]
    pub profile: super::RagProfileView,

    /// Custom entity-type → hex color map for graph visualization (SPEC-102).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type_colors: Option<std::collections::HashMap<String, String>>,

    /// Relation type allow-list (SPEC-114). None/empty means free-form relations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_types: Option<Vec<String>>,

    /// When true, unknown relations remap (default true when list present).
    pub relation_types_strict: bool,

    /// Domain preset id (SPEC-114), if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kg_schema_preset: Option<String>,

    /// Typed edge constraints (SPEC-114b). Absent/empty ⇒ unconstrained endpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_edges: Option<Vec<super::RelationEdgeDto>>,

    /// SPEC-109: workspace default reasoning effort.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reasoning_effort: Option<String>,

    /// SPEC-109: per-role LLM overrides (incl. reasoning_effort).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_roles: Option<serde_json::Value>,

    // === Vision extract (SPEC-015V) ===
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_extract_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_extract_charts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_extract_figures: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_page_system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_image_system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_chart_system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_figure_system_prompt: Option<String>,

    /// Aggregate stats for this workspace.
    ///
    /// Only populated when the caller asks for them
    /// (`GET /tenants/{tenant_id}/workspaces?include_stats=true`), so existing
    /// clients see byte-identical payloads. Avoids the N+1 round-trip of
    /// listing workspaces then calling `/workspaces/{id}/stats` for each one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<WorkspaceStatsResponse>,

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
    /// Total tenants (not page length). SPEC-140.
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
    /// Total matching workspaces for the tenant (not page length). SPEC-140.
    pub total: usize,
    /// Current offset.
    pub offset: usize,
    /// Page size limit.
    pub limit: usize,
}

// ── Pagination and Stats ──────────────────────────────────────────────────

/// Query params for `GET /tenants/{tenant_id}/workspaces`.
///
/// Flattens [`PaginationParams`] and adds an opt-in `include_stats` flag.
/// Default is `false`, so the endpoint's behaviour is unchanged for callers
/// that do not ask for stats.
///
/// NOTE: the pagination fields are declared explicitly rather than via
/// `#[serde(flatten)] PaginationParams`. Flattening breaks query-string
/// deserialization — values arrive as strings and `usize` then fails with
/// `invalid type: string "1", expected usize`, turning `?limit=1` into a 400.
#[derive(Debug, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListWorkspacesParams {
    /// Offset (default 0).
    #[serde(default)]
    pub offset: usize,
    /// Limit (default 20, max 100).
    #[serde(default = "workspaces_default_limit")]
    pub limit: usize,

    /// When true, each item carries its `stats` object (served from the same
    /// 60s cache as `/workspaces/{id}/stats`).
    #[serde(default)]
    pub include_stats: bool,
}

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
    /// True when stats were served from cache because live fetch timed out under load.
    pub stale: bool,
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
