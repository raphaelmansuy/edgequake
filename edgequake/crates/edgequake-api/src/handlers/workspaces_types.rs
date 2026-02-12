//! DTOs for workspace management API endpoints.
//!
//! This module contains all data transfer objects used in tenant and workspace management,
//! including create/update requests, responses, and statistics.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to create a new tenant.
///
/// ## Model Configuration (SPEC-032)
///
/// When creating a tenant, you can specify default LLM and embedding models
/// that will be inherited by all new workspaces within this tenant.
///
/// **LLM Examples (for knowledge graph generation, summarization):**
/// - OpenAI: `"gpt-4o-mini"`, `"gpt-4o"`
/// - Ollama: `"gemma3:12b"`, `"llama3.2"`
/// - LM Studio: `"gemma-3n-e4b-it-mlxmodel"`
///
/// **Embedding Examples:**
/// - OpenAI: `"text-embedding-3-small"` (1536 dims), `"text-embedding-3-large"` (3072 dims)
/// - Ollama: `"embeddinggemma:latest"` (768 dims), `"nomic-embed-text"` (768 dims)
/// - LM Studio: `"nomic-ai/nomic-embed-text-v1.5"` (768 dims)
///
/// **Model ID Format:**
/// Models can be specified as `model_name` or `provider/model_name`:
/// - `"gemma3:12b"` - auto-detects provider as "ollama"
/// - `"ollama/gemma3:12b"` - explicit provider
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    /// Tenant name.
    pub name: String,
    /// URL-friendly slug (auto-generated if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Plan type (free, basic, pro, enterprise).
    pub plan: Option<String>,

    // === Default LLM Configuration (SPEC-032) ===
    /// Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, uses server default from models.toml or EDGEQUAKE_DEFAULT_LLM_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_llm_model: Option<String>,

    /// Default LLM provider for new workspaces ("openai", "ollama", "lmstudio").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name or uses EDGEQUAKE_DEFAULT_LLM_PROVIDER.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_llm_provider: Option<String>,

    // === Default Embedding Configuration (SPEC-032) ===
    /// Default embedding model for new workspaces (e.g., "text-embedding-3-small").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, uses server default from models.toml or EDGEQUAKE_DEFAULT_EMBEDDING_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_model: Option<String>,

    /// Default embedding provider for new workspaces ("openai", "ollama", "lmstudio").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name or uses EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_provider: Option<String>,

    /// Default embedding dimension for new workspaces (e.g., 1536 for OpenAI, 768 for Ollama).
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name or uses EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_dimension: Option<usize>,
}

/// Request to update a tenant.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateTenantRequest {
    /// New tenant name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New plan.
    pub plan: Option<String>,
    /// Whether the tenant is active.
    pub is_active: Option<bool>,
}

/// Request to create a new workspace.
///
/// ## Model Configuration (SPEC-032)
///
/// When creating a workspace, you can specify both LLM and embedding models.
/// If not provided, server defaults are used (configurable via env vars or models.toml).
///
/// **LLM Examples (for knowledge graph generation, summarization):**
/// - OpenAI: `"gpt-4o-mini"`, `"gpt-4o"`
/// - Ollama: `"gemma3:12b"`, `"llama3.2"`
/// - LM Studio: `"gemma-3n-e4b-it-mlxmodel"`
///
/// **Embedding Examples:**
/// - OpenAI: `"text-embedding-3-small"` (1536 dims), `"text-embedding-3-large"` (3072 dims)
/// - Ollama: `"embeddinggemma:latest"` (768 dims), `"nomic-embed-text"` (768 dims)
/// - LM Studio: `"nomic-ai/nomic-embed-text-v1.5"` (768 dims)
///
/// **Model ID Format:**
/// Models can be specified as `model_name` or `provider/model_name`:
/// - `"gemma3:12b"` - auto-detects provider as "ollama"
/// - `"ollama/gemma3:12b"` - explicit provider
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceApiRequest {
    /// Workspace name.
    pub name: String,
    /// URL-friendly slug (auto-generated if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Maximum number of documents.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model for knowledge graph generation, summarization, entity extraction.
    /// Format: "model_name" or "provider/model_name" (e.g., "gemma3:12b", "ollama/gemma3:12b").
    /// If not provided, uses server default from models.toml or EDGEQUAKE_DEFAULT_LLM_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// LLM provider ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected from llm_model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// If not provided, uses server default from EDGEQUAKE_DEFAULT_EMBEDDING_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Embedding provider ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected from embedding_model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,

    /// Embedding vector dimension override.
    /// If not provided, auto-detected from embedding_model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<usize>,
}

/// Request to update a workspace.
///
/// ## Model Configuration Updates (SPEC-032)
///
/// Changing LLM provider/model is safe and takes effect immediately for new ingestions.
/// Changing embedding provider/model requires rebuilding vectors (use rebuild-embeddings endpoint).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkspaceApiRequest {
    /// New workspace name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: Option<bool>,
    /// Maximum number of documents.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// Update LLM model (takes effect on next ingestion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Update LLM provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Update embedding model.
    /// WARNING: Requires vector rebuild - use rebuild-embeddings endpoint after updating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Update embedding provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,

    /// Update embedding dimension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<usize>,
}

// ============================================================================
// Response DTOs
// ============================================================================

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

    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

// ============================================================================
// List Response DTOs
// ============================================================================

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

// ============================================================================
// Pagination and Stats DTOs
// ============================================================================

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

// ============================================================================
// Rebuild Embeddings DTOs (SPEC-032)
// ============================================================================

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
    /// Warning message if chunk size exceeds model context length.
    /// REQ-25: Critical invariant - chunks must fit model's input limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_warning: Option<String>,
}

// ============================================================================
// Reprocess All Documents DTOs (SPEC-032 Focus Area 5)
// ============================================================================

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

// ============================================================================
// Rebuild Knowledge Graph DTOs (LLM Model Change)
// ============================================================================

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
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tenant_request_serialization() {
        let req = CreateTenantRequest {
            name: "Acme Corp".to_string(),
            slug: Some("acme".to_string()),
            description: Some("Test tenant".to_string()),
            plan: Some("pro".to_string()),
            default_llm_model: Some("gemma3:12b".to_string()),
            default_llm_provider: Some("ollama".to_string()),
            default_embedding_model: Some("text-embedding-3-small".to_string()),
            default_embedding_provider: Some("openai".to_string()),
            default_embedding_dimension: Some(1536),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Acme Corp"));
        assert!(json.contains("acme"));
        assert!(json.contains("gemma3:12b"));
        assert!(json.contains("ollama"));
    }

    #[test]
    fn test_update_tenant_request_serialization() {
        let req = UpdateTenantRequest {
            name: Some("New Name".to_string()),
            description: None,
            plan: Some("enterprise".to_string()),
            is_active: Some(false),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("New Name"));
        assert!(json.contains("enterprise"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_create_workspace_api_request_serialization() {
        let req = CreateWorkspaceApiRequest {
            name: "Main Workspace".to_string(),
            slug: Some("main".to_string()),
            description: Some("Primary workspace".to_string()),
            max_documents: Some(1000),
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Main Workspace"));
        assert!(json.contains("1000"));
    }

    #[test]
    fn test_update_workspace_api_request_serialization() {
        let req = UpdateWorkspaceApiRequest {
            name: Some("Updated Workspace".to_string()),
            description: None,
            is_active: Some(true),
            max_documents: Some(2000),
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Updated Workspace"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_tenant_response_serialization() {
        let response = TenantResponse {
            id: Uuid::nil(),
            name: "Test Tenant".to_string(),
            slug: "test".to_string(),
            plan: "free".to_string(),
            is_active: true,
            max_workspaces: 10, // SPEC-028: Updated to reflect new Free tier limit
            default_llm_model: "gemma3:12b".to_string(),
            default_llm_provider: "ollama".to_string(),
            default_llm_full_id: "ollama/gemma3:12b".to_string(),
            default_embedding_model: "text-embedding-3-small".to_string(),
            default_embedding_provider: "openai".to_string(),
            default_embedding_dimension: 1536,
            default_embedding_full_id: "openai/text-embedding-3-small".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Tenant"));
        assert!(json.contains("\"max_workspaces\":10")); // SPEC-028
        assert!(json.contains("\"default_llm_model\":\"gemma3:12b\""));
        assert!(json.contains("\"default_embedding_dimension\":1536"));
    }

    #[test]
    fn test_workspace_response_serialization() {
        let response = WorkspaceResponse {
            id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            name: "Test Workspace".to_string(),
            slug: "test".to_string(),
            description: Some("A test workspace".to_string()),
            is_active: true,
            max_documents: Some(100),
            // SPEC-032: LLM configuration
            llm_model: "gemma3:12b".to_string(),
            llm_provider: "ollama".to_string(),
            llm_full_id: "ollama/gemma3:12b".to_string(),
            // SPEC-032: Embedding configuration
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_provider: "openai".to_string(),
            embedding_dimension: 1536,
            embedding_full_id: "openai/text-embedding-3-small".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Workspace"));
        assert!(json.contains("A test workspace"));
        assert!(json.contains("\"llm_model\":\"gemma3:12b\""));
        assert!(json.contains("\"llm_full_id\":\"ollama/gemma3:12b\""));
        assert!(json.contains("\"embedding_model\":\"text-embedding-3-small\""));
        assert!(json.contains("\"embedding_dimension\":1536"));
    }

    #[test]
    fn test_tenant_list_response_serialization() {
        let response = TenantListResponse {
            items: vec![],
            total: 42,
            offset: 0,
            limit: 20,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":42"));
        assert!(json.contains("\"limit\":20"));
    }

    #[test]
    fn test_workspace_list_response_serialization() {
        let response = WorkspaceListResponse {
            items: vec![],
            total: 15,
            offset: 10,
            limit: 5,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":15"));
        assert!(json.contains("\"offset\":10"));
    }

    #[test]
    fn test_pagination_params_defaults() {
        let json = "{}";
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.offset, 0);
        assert_eq!(params.limit, 20);
    }

    #[test]
    fn test_workspace_stats_response_serialization() {
        let response = WorkspaceStatsResponse {
            workspace_id: Uuid::nil(),
            document_count: 10,
            entity_count: 50,
            relationship_count: 25,
            entity_type_count: 5,
            chunk_count: 100,
            embedding_count: 80,
            storage_bytes: 1024 * 1024,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"document_count\":10"));
        assert!(json.contains("\"entity_count\":50"));
        assert!(json.contains("\"embedding_count\":80"));
        assert!(json.contains("\"storage_bytes\":1048576"));
    }
}
