//! Workspace and tenant management handlers.
//!
//! # Implements
//!
//! - **UC0301**: Create Workspace
//! - **UC0302**: List Workspaces
//! - **UC0303**: Switch Workspace
//! - **UC0304**: Delete Workspace
//! - **FEAT0701**: Multi-Tenancy Support
//! - **FEAT0702**: Workspace Isolation
//! - **FEAT0401**: REST API Service
//!
//! # Enforces
//!
//! - **BR0201**: Tenant isolation (all operations scoped to tenant)
//! - **BR0202**: Workspace quotas enforced by plan
//! - **BR0203**: Resource limits per workspace
//! - **BR0401**: Authentication required
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | `/api/v1/tenants` | [`create_tenant`] | Create new tenant |
//! | GET | `/api/v1/tenants` | [`list_tenants`] | List all tenants |
//! | POST | `/api/v1/workspaces` | [`create_workspace`] | Create workspace |
//! | GET | `/api/v1/workspaces` | [`list_workspaces`] | List workspaces |
//! | DELETE | `/api/v1/workspaces/:id` | [`delete_workspace`] | Delete workspace |
//!
//! # WHY: Hierarchical Multi-Tenancy
//!
//! EdgeQuake uses a two-level hierarchy:
//! - **Tenant**: Organization/company level (billing, limits, users)
//! - **Workspace**: Project/team level (isolated knowledge graphs)
//!
//! This enables:
//! - SaaS deployment with multiple customers
//! - Per-project knowledge isolation
//! - Usage tracking and billing per tenant
//!
//! # Module Organization (SRP)
//!
//! - `helpers`: Stats cache, slug generation, workspace-to-response conversion
//! - `tenants`: Tenant CRUD (create, list, get, update, delete)
//! - `workspace_crud`: Workspace CRUD (create, list, get, update, delete)
//! - `stats`: Workspace statistics, metrics history, metrics snapshots
//! - `bulk_ops`: Rebuild embeddings, rebuild knowledge graph, reprocess all docs

// Sub-modules: each owns a single responsibility
mod bulk_ops;
mod helpers;
mod stats;
mod tenants;
mod workspace_crud;

// Re-export all public items (includes utoipa __path_* structs for OpenAPI)
pub use bulk_ops::*;
pub use helpers::invalidate_workspace_stats_cache;
pub use stats::*;
pub use tenants::*;
pub use workspace_crud::*;

// Re-export DTOs from workspaces_types module
pub use crate::handlers::workspaces_types::*;

#[cfg(test)]
mod tests {
    use super::helpers::generate_slug;
    use super::*;

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("My Knowledge Base"), "my-knowledge-base");
        assert_eq!(generate_slug("Test 123!"), "test-123");
        assert_eq!(generate_slug("  multiple   spaces  "), "multiple-spaces");
    }

    #[test]
    fn test_generate_slug_edge_cases() {
        assert_eq!(generate_slug(""), "");
        assert_eq!(generate_slug("UPPERCASE"), "uppercase");
        assert_eq!(generate_slug("already-slug"), "already-slug");
        assert_eq!(generate_slug("123"), "123");
    }

    #[test]
    fn test_create_tenant_request_deserialization() {
        let json = r#"{"name": "Test Tenant"}"#;
        let request: Result<CreateTenantRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.name, "Test Tenant");
        assert!(req.slug.is_none());
        assert!(req.plan.is_none());
    }

    #[test]
    fn test_update_tenant_request_partial() {
        let json = r#"{"name": "Updated Name"}"#;
        let request: Result<UpdateTenantRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.name, Some("Updated Name".to_string()));
        assert!(req.is_active.is_none());
    }

    #[test]
    fn test_create_workspace_request_deserialization() {
        let json = r#"{"name": "Test Workspace", "description": "A test workspace"}"#;
        let request: Result<CreateWorkspaceApiRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.name, "Test Workspace");
        assert_eq!(req.description, Some("A test workspace".to_string()));
    }

    #[test]
    fn test_pagination_params_defaults() {
        let json = r#"{}"#;
        let params: Result<PaginationParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());
        let p = params.unwrap();
        // Default values from serde(default)
        assert_eq!(p.offset, 0);
        assert_eq!(p.limit, 20);
    }

    #[test]
    fn test_tenant_response_serialization() {
        let response = TenantResponse {
            id: uuid::Uuid::new_v4(),
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            plan: "free".to_string(),
            is_active: true,
            max_workspaces: 5,
            default_llm_model: "gemma3:12b".to_string(),
            default_llm_provider: "ollama".to_string(),
            default_llm_full_id: "ollama/gemma3:12b".to_string(),
            default_embedding_model: "text-embedding-3-small".to_string(),
            default_embedding_provider: "openai".to_string(),
            default_embedding_dimension: 1536,
            default_embedding_full_id: "openai/text-embedding-3-small".to_string(),
            default_vision_llm_model: None,
            default_vision_llm_provider: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("test-tenant"));
        assert!(json_str.contains("gemma3:12b"));
        assert!(json_str.contains("text-embedding-3-small"));
    }

    #[test]
    fn test_workspace_stats_response_serialization() {
        let response = WorkspaceStatsResponse {
            workspace_id: uuid::Uuid::new_v4(),
            document_count: 100,
            entity_count: 500,
            relationship_count: 200,
            entity_type_count: 15,
            chunk_count: 1000,
            embedding_count: 800,
            storage_bytes: 1024 * 1024,
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("\"document_count\":100"));
        assert!(json_str.contains("\"embedding_count\":800"));
    }
}
