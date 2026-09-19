//! DTOs for workspace management API endpoints.
//!
//! This module contains all data transfer objects used in tenant and workspace
//! management, including create/update requests, responses, and statistics.
//!
//! ## Sub-modules
//!
//! | Module      | Purpose                                           |
//! |-------------|---------------------------------------------------|
//! | `requests`  | Create / update DTOs for tenants and workspaces   |
//! | `responses` | Response DTOs, list wrappers, pagination, stats   |
//! | `rebuild`   | Rebuild-embeddings, reprocess, rebuild-KG DTOs    |

mod copy;
mod map;
mod rebuild;
mod requests;
mod responses;

pub use copy::*;
pub use map::workspace_to_response;
pub use rebuild::*;
pub use requests::*;
pub use responses::*;

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_create_tenant_request_serialization() {
        let req = CreateTenantRequest {
            name: "Acme Corp".to_string(),
            slug: Some("acme".to_string()),
            description: Some("Test tenant".to_string()),
            plan: Some("pro".to_string()),
            default_llm_model: Some("gemma4:latest".to_string()),
            default_llm_provider: Some("ollama".to_string()),
            default_embedding_model: Some("text-embedding-3-small".to_string()),
            default_embedding_provider: Some("openai".to_string()),
            default_embedding_dimension: Some(1536),
            default_vision_llm_model: Some("gpt-4o".to_string()),
            default_vision_llm_provider: Some("openai".to_string()),
            default_reasoning_effort: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Acme Corp"));
        assert!(json.contains("acme"));
        assert!(json.contains("gemma4:latest"));
        assert!(json.contains("ollama"));
    }

    #[test]
    fn test_update_tenant_request_serialization() {
        let req = UpdateTenantRequest {
            name: Some("New Name".to_string()),
            description: None,
            plan: Some("enterprise".to_string()),
            is_active: Some(false),
            pdf_parser_backend: None,
            default_llm_model: None,
            default_llm_provider: None,
            default_embedding_model: None,
            default_embedding_provider: None,
            default_embedding_dimension: None,
            default_vision_llm_model: None,
            default_vision_llm_provider: None,
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
            vision_llm_model: None,
            vision_llm_provider: None,
            pdf_parser_backend: None,
            entity_types: None,
            entity_types_strict: None,
            extraction_language: None,
            chunking_mode: None,
            chunk_token_size: None,
            chunk_overlap_token_size: None,
            extract_budget_mode: None,
            extract_max_entities: None,
            extract_max_records: None,
            entity_type_colors: None,
            relation_types: None,
            relation_types_strict: None,
            kg_schema_preset: None,
            relation_edges: None,
            default_reasoning_effort: None,
            llm_roles: None,
            vision_extract_images: None,
            vision_extract_charts: None,
            vision_extract_figures: None,
            vision_page_system_prompt: None,
            vision_image_system_prompt: None,
            vision_chart_system_prompt: None,
            vision_figure_system_prompt: None,
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
            vision_llm_provider: None,
            vision_llm_model: None,
            pdf_parser_backend: None,
            entity_types: Some(vec!["PERSON".to_string(), "ORGANIZATION".to_string()]),
            entity_types_strict: None,
            extraction_language: None,
            chunking_mode: None,
            chunk_token_size: None,
            chunk_overlap_token_size: None,
            extract_budget_mode: None,
            extract_max_entities: None,
            extract_max_records: None,
            entity_type_colors: None,
            relation_types: None,
            relation_types_strict: None,
            kg_schema_preset: None,
            relation_edges: None,
            default_reasoning_effort: None,
            llm_roles: None,
            vision_extract_images: None,
            vision_extract_charts: None,
            vision_extract_figures: None,
            vision_page_system_prompt: None,
            vision_image_system_prompt: None,
            vision_chart_system_prompt: None,
            vision_figure_system_prompt: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Updated Workspace"));
        assert!(json.contains("PERSON"));
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
            default_llm_model: "gemma4:latest".to_string(),
            default_llm_provider: "ollama".to_string(),
            default_llm_full_id: "ollama/gemma4:latest".to_string(),
            default_embedding_model: "text-embedding-3-small".to_string(),
            default_embedding_provider: "openai".to_string(),
            default_embedding_dimension: 1536,
            default_embedding_full_id: "openai/text-embedding-3-small".to_string(),
            default_vision_llm_model: None,
            default_vision_llm_provider: None,
            default_reasoning_effort: None,
            pdf_parser_backend: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Tenant"));
        assert!(json.contains("\"max_workspaces\":10")); // SPEC-028
        assert!(json.contains("\"default_llm_model\":\"gemma4:latest\""));
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
            llm_model: "gemma4:latest".to_string(),
            llm_provider: "ollama".to_string(),
            llm_full_id: "ollama/gemma4:latest".to_string(),
            resolved_llm_provider: None,
            resolved_llm_model: None,
            llm_resolution_source: None,
            // SPEC-032: Embedding configuration
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_provider: "openai".to_string(),
            embedding_dimension: 1536,
            embedding_full_id: "openai/text-embedding-3-small".to_string(),
            resolved_embedding_provider: None,
            resolved_embedding_model: None,
            resolved_embedding_dimension: None,
            embedding_resolution_source: None,
            vision_llm_provider: None,
            vision_llm_model: None,
            resolved_vision_llm_provider: None,
            resolved_vision_llm_model: None,
            vision_llm_resolution_source: None,
            pdf_parser_backend: None,
            entity_types: None,
            entity_types_strict: true,
            extraction_language: None,
            chunking_mode: None,
            chunk_token_size: None,
            chunk_overlap_token_size: None,
            extract_budget_mode: None,
            extract_max_entities: None,
            extract_max_records: None,
            entity_type_colors: None,
            relation_types: None,
            relation_types_strict: true,
            kg_schema_preset: None,
            relation_edges: None,
            stats: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            default_reasoning_effort: None,
            llm_roles: None,
            vision_extract_images: None,
            vision_extract_charts: None,
            vision_extract_figures: None,
            vision_page_system_prompt: None,
            vision_image_system_prompt: None,
            vision_chart_system_prompt: None,
            vision_figure_system_prompt: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test Workspace"));
        assert!(json.contains("A test workspace"));
        assert!(json.contains("\"llm_model\":\"gemma4:latest\""));
        assert!(json.contains("\"llm_full_id\":\"ollama/gemma4:latest\""));
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
            stale: false,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"document_count\":10"));
        assert!(json.contains("\"entity_count\":50"));
        assert!(json.contains("\"embedding_count\":80"));
        assert!(json.contains("\"storage_bytes\":1048576"));
    }
}
