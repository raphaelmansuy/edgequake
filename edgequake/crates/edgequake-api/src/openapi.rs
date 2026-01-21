//! OpenAPI documentation.

use utoipa::OpenApi;

use crate::handlers;

/// OpenAPI documentation.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "EdgeQuake API",
        version = "0.1.0",
        description = "High-performance RAG system with Knowledge Graph",
        license(name = "MIT OR Apache-2.0"),
        contact(
            name = "EdgeQuake Team"
        )
    ),
    paths(
        handlers::health_check,
        handlers::readiness_check,
        handlers::liveness_check,
        handlers::get_metrics,
        handlers::upload_document,
        handlers::list_documents,
        handlers::execute_query,
        handlers::stream_query,
        handlers::get_graph,
        handlers::stream_graph,
        handlers::get_node,
        handlers::search_labels,
        // Entity operations (Phase 2)
        handlers::list_entities,
        handlers::create_entity,
        handlers::get_entity,
        handlers::update_entity,
        handlers::delete_entity,
        handlers::entity_exists,
        handlers::merge_entities,
        handlers::get_entity_neighborhood,
        // Relationship operations (Phase 2)
        handlers::list_relationships,
        handlers::create_relationship,
        handlers::get_relationship,
        handlers::update_relationship,
        handlers::delete_relationship,
        // Authentication (Phase 3)
        handlers::login,
        handlers::refresh_token,
        handlers::logout,
        handlers::get_me,
        handlers::create_user,
        handlers::list_users,
        handlers::get_user,
        handlers::delete_user,
        handlers::create_api_key,
        handlers::list_api_keys,
        handlers::revoke_api_key,
        // Models Configuration (SPEC-032)
        handlers::list_models,
        handlers::list_llm_models,
        handlers::list_embedding_models,
        handlers::get_provider,
        handlers::get_model,
        handlers::check_providers_health,
    ),
    components(schemas(
        handlers::HealthResponse,
        handlers::ComponentHealth,
        handlers::UploadDocumentRequest,
        handlers::UploadDocumentResponse,
        handlers::ListDocumentsResponse,
        handlers::DocumentSummary,
        handlers::QueryRequest,
        handlers::QueryResponse,
        handlers::SourceReference,
        handlers::QueryStats,
        handlers::StreamQueryRequest,
        handlers::KnowledgeGraphResponse,
        handlers::GraphNodeResponse,
        handlers::GraphEdgeResponse,
        handlers::GraphQueryParams,
        handlers::GraphStreamQueryParams,
        handlers::GraphStreamEvent,
        handlers::SearchLabelsQuery,
        handlers::SearchLabelsResponse,
        // Entity schemas (Phase 2)
        handlers::CreateEntityRequest,
        handlers::CreateEntityResponse,
        handlers::UpdateEntityRequest,
        handlers::UpdateEntityResponse,
        handlers::DeleteEntityResponse,
        handlers::DeleteEntityQuery,
        handlers::EntityExistsQuery,
        handlers::EntityExistsResponse,
        handlers::MergeEntitiesRequest,
        handlers::MergeEntitiesResponse,
        handlers::MergeDetails,
        handlers::EntityResponse,
        handlers::GetEntityResponse,
        handlers::RelationshipsInfo,
        handlers::RelationshipSummary,
        handlers::EntityStatistics,
        handlers::ChangesSummary,
        handlers::ListEntitiesQuery,
        handlers::ListEntitiesResponse,
        handlers::EntityNeighborhoodQuery,
        handlers::EntityNeighborhoodResponse,
        handlers::NeighborhoodNode,
        handlers::NeighborhoodEdge,
        // Relationship schemas (Phase 2)
        handlers::CreateRelationshipRequest,
        handlers::CreateRelationshipResponse,
        handlers::UpdateRelationshipRequest,
        handlers::UpdateRelationshipResponse,
        handlers::DeleteRelationshipResponse,
        handlers::RelationshipResponse,
        handlers::GetRelationshipResponse,
        handlers::ListRelationshipsQuery,
        handlers::ListRelationshipsResponse,
        handlers::RelationshipEntities,
        handlers::EntitySummary,
        handlers::RelationshipChangesSummary,
        // Authentication schemas (Phase 3)
        handlers::LoginRequest,
        handlers::LoginResponse,
        handlers::UserInfo,
        handlers::RefreshTokenRequest,
        handlers::RefreshTokenResponse,
        handlers::CreateUserRequest,
        handlers::CreateUserResponse,
        handlers::CreateApiKeyRequest,
        handlers::CreateApiKeyResponse,
        handlers::ApiKeySummary,
        handlers::ListApiKeysResponse,
        handlers::RevokeApiKeyResponse,
        handlers::GetMeResponse,
        // Models Configuration schemas (SPEC-032)
        handlers::ModelsListResponse,
        handlers::ProviderResponse,
        handlers::ModelResponse,
        handlers::ModelCapabilitiesResponse,
        handlers::ModelCostResponse,
        handlers::ProviderHealthResponse,
        handlers::LlmModelsResponse,
        handlers::LlmModelItem,
        handlers::EmbeddingModelsResponse,
        handlers::EmbeddingModelItem,
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Observability", description = "Metrics and monitoring endpoints (Phase 3)"),
        (name = "Documents", description = "Document ingestion endpoints"),
        (name = "Query", description = "Query execution endpoints"),
        (name = "Graph", description = "Knowledge graph exploration endpoints"),
        (name = "Entities", description = "Entity CRUD operations (Phase 2)"),
        (name = "Relationships", description = "Relationship CRUD operations (Phase 2)"),
        (name = "Authentication", description = "User authentication and session management (Phase 3)"),
        (name = "User Management", description = "User administration endpoints (Phase 3)"),
        (name = "API Keys", description = "API key management endpoints (Phase 3)"),
        (name = "Models", description = "Model configuration and capability discovery (SPEC-032)"),
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security addon for OpenAPI documentation.
/// Also adds tenant/workspace header documentation.
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
            components.add_security_scheme(
                "api_key",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new("X-API-Key"),
                    ),
                ),
            );
            // SPEC-032: Add X-Tenant-ID header as security scheme for documentation
            components.add_security_scheme(
                "tenant_id",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new("X-Tenant-ID"),
                    ),
                ),
            );
            // SPEC-032: Add X-Workspace-ID header as security scheme for documentation
            components.add_security_scheme(
                "workspace_id",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new("X-Workspace-ID"),
                    ),
                ),
            );
        }

        // SPEC-032: Add description about context headers in API description
        if let Some(info) = Some(&mut openapi.info) {
            let current_desc = info.description.clone().unwrap_or_default();
            info.description = Some(format!(
                "{}\n\n## Context Headers (SPEC-032)\n\n\
                 Most endpoints require tenant and workspace context via headers:\n\n\
                 - **X-Tenant-ID**: UUID of the tenant (organization). Required for multi-tenant operations.\n\
                 - **X-Workspace-ID**: UUID of the workspace. Required for document/query operations.\n\n\
                 These headers are automatically set by the WebUI when a user selects a tenant/workspace.\n\n\
                 Example:\n\
                 ```\n\
                 X-Tenant-ID: 00000000-0000-0000-0000-000000000001\n\
                 X-Workspace-ID: 00000000-0000-0000-0000-000000000002\n\
                 ```",
                current_desc
            ));
        }
    }
}
