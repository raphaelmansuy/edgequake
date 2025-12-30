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
        handlers::create_entity,
        handlers::get_entity,
        handlers::update_entity,
        handlers::delete_entity,
        handlers::entity_exists,
        handlers::merge_entities,
        // Relationship operations (Phase 2)
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
        // Relationship schemas (Phase 2)
        handlers::CreateRelationshipRequest,
        handlers::CreateRelationshipResponse,
        handlers::UpdateRelationshipRequest,
        handlers::UpdateRelationshipResponse,
        handlers::DeleteRelationshipResponse,
        handlers::RelationshipResponse,
        handlers::GetRelationshipResponse,
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
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security addon for OpenAPI documentation.
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
        }
    }
}
