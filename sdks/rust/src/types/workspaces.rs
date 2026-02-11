//! Workspace types.

use serde::{Deserialize, Serialize};

/// Create workspace request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    #[serde(default)]
    pub created_at: Option<String>,
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
