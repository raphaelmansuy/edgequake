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
/// ## Embedding Configuration (SPEC-032)
///
/// When creating a workspace, you can specify the embedding model to use.
/// If not provided, server defaults are used (configurable via env vars).
///
/// **Examples:**
/// - OpenAI: `"text-embedding-3-small"` (1536 dims), `"text-embedding-3-large"` (3072 dims)
/// - Ollama: `"embeddinggemma:latest"` (768 dims), `"nomic-embed-text"` (768 dims)
/// - LM Studio: `"nomic-ai/nomic-embed-text-v1.5"` (768 dims)
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
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Tenant response DTO.
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
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Workspace response DTO.
///
/// Includes embedding configuration (SPEC-032) for transparency.
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

    // === Embedding Configuration (SPEC-032) ===

    /// Embedding model used for this workspace.
    pub embedding_model: String,
    /// Embedding provider (openai, ollama, lmstudio).
    pub embedding_provider: String,
    /// Embedding vector dimension.
    pub embedding_dimension: usize,

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
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceStatsResponse {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Number of documents.
    pub document_count: usize,
    /// Number of entities.
    pub entity_count: usize,
    /// Number of relationships.
    pub relationship_count: usize,
    /// Number of chunks.
    pub chunk_count: usize,
    /// Storage used in bytes.
    pub storage_bytes: u64,
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
    /// Number of vectors cleared.
    pub vectors_cleared: usize,
    /// New embedding model (after update).
    pub embedding_model: String,
    /// New embedding provider (after update).
    pub embedding_provider: String,
    /// New embedding dimension (after update).
    pub embedding_dimension: usize,
    /// Estimated time to complete (seconds).
    pub estimated_time_seconds: Option<u64>,
    /// Background job ID for tracking (if async).
    pub job_id: Option<String>,
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
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Acme Corp"));
        assert!(json.contains("acme"));
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
            max_workspaces: 5,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Tenant"));
        assert!(json.contains("\"max_workspaces\":5"));
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
            // SPEC-032: Embedding configuration
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_provider: "openai".to_string(),
            embedding_dimension: 1536,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Workspace"));
        assert!(json.contains("A test workspace"));
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
            chunk_count: 100,
            storage_bytes: 1024 * 1024,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"document_count\":10"));
        assert!(json.contains("\"entity_count\":50"));
        assert!(json.contains("\"storage_bytes\":1048576"));
    }
}
