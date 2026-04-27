//! Workspace types.

use serde::{Deserialize, Serialize};

/// Create workspace request.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // LLM configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,
    // Embedding configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<u32>,
    // Vision LLM for PDF image extraction (SPEC-041). Inherits from tenant if not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_provider: Option<String>,
}

/// Workspace summary.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    // LLM configuration.
    #[serde(default)]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub llm_provider: Option<String>,
    #[serde(default)]
    pub llm_full_id: Option<String>,
    // Embedding configuration.
    #[serde(default)]
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub embedding_dimension: Option<u32>,
    #[serde(default)]
    pub embedding_full_id: Option<String>,
    // Vision LLM (SPEC-041) – only present when configured or inherited from tenant.
    #[serde(default)]
    pub vision_llm_model: Option<String>,
    #[serde(default)]
    pub vision_llm_provider: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Paginated workspace list from `GET /api/v1/tenants/{tenant_id}/workspaces`.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceListResponse {
    pub items: Vec<WorkspaceInfo>,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub limit: usize,
}

/// Workspace statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceStats {
    pub workspace_id: String,
    #[serde(default)]
    pub document_count: u32,
    #[serde(default)]
    pub entity_count: u32,
    #[serde(default)]
    pub relationship_count: u32,
    #[serde(default)]
    pub chunk_count: u32,
    #[serde(default)]
    pub query_count: u32,
    #[serde(default)]
    pub storage_size_bytes: u64,
}

/// Rebuild response.
#[derive(Debug, Clone, Deserialize)]
pub struct RebuildResponse {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
}
