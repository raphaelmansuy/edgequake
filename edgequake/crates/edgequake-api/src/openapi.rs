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
        handlers::upload_document,
        handlers::list_documents,
        handlers::execute_query,
        handlers::stream_query,
        handlers::get_graph,
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
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Documents", description = "Document ingestion endpoints"),
        (name = "Query", description = "Query execution endpoints"),
        (name = "Graph", description = "Knowledge graph exploration endpoints"),
        (name = "Entities", description = "Entity CRUD operations (Phase 2)"),
        (name = "Relationships", description = "Relationship CRUD operations (Phase 2)"),
    )
)]
pub struct ApiDoc;
