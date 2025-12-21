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
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Documents", description = "Document ingestion endpoints"),
        (name = "Query", description = "Query execution endpoints"),
        (name = "Graph", description = "Knowledge graph exploration endpoints"),
    )
)]
pub struct ApiDoc;
