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
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Workspace"));
        assert!(json.contains("A test workspace"));
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
