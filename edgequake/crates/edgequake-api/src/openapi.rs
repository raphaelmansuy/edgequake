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
        license(name = "Apache-2.0"),
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
        // Chat (SDK-aligned)
        handlers::chat::completion::chat_completion,
        handlers::chat::streaming::chat_completion_stream,
        // Conversations & Folders (SDK-aligned)
        handlers::list_conversations,
        handlers::create_conversation,
        handlers::get_conversation,
        handlers::update_conversation,
        handlers::delete_conversation,
        handlers::list_messages,
        handlers::create_message,
        handlers::bulk_delete_conversations,
        handlers::list_folders,
        handlers::create_folder,
        handlers::update_folder,
        handlers::delete_folder,
        // Pipeline (SDK-aligned)
        handlers::get_pipeline_status,
        handlers::cancel_pipeline,
        handlers::get_queue_metrics,
        // Tasks (SDK-aligned)
        handlers::list_tasks,
        handlers::get_task,
        handlers::cancel_task,
        handlers::retry_task,
        // Costs (SDK-aligned)
        handlers::get_cost_summary,
        handlers::get_model_pricing,
        handlers::estimate_cost,
        // Tenants & Workspaces (SDK-aligned)
        handlers::create_tenant,
        handlers::list_tenants,
        handlers::get_tenant,
        handlers::update_tenant,
        handlers::delete_tenant,
        handlers::create_workspace,
        handlers::list_workspaces,
        handlers::get_workspace,
        handlers::update_workspace,
        handlers::delete_workspace,
        handlers::get_workspace_stats,
        // Lineage & Provenance
        handlers::get_chunk_detail,
        handlers::get_entity_provenance,
        handlers::get_entity_lineage,
        handlers::get_document_lineage,
        handlers::get_document_full_lineage,
        handlers::get_document_metadata,
        handlers::export_document_lineage,
        handlers::get_chunk_lineage,
        // PDF Upload
        handlers::upload_pdf_document,
        handlers::get_pdf_status,
        handlers::list_pdfs,
        handlers::delete_pdf,
        handlers::get_pdf_progress,
        handlers::get_pdf_content,
    ),
    components(schemas(
        handlers::HealthResponse,
        handlers::ComponentHealth,
        handlers::BuildInfo,
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
        // Chat schemas (SDK-aligned)
        handlers::ChatCompletionRequest,
        handlers::ChatCompletionResponse,
        // Conversations & Folders schemas (SDK-aligned)
        handlers::ConversationResponse,
        handlers::MessageResponse,
        handlers::FolderResponse,
        handlers::PaginatedConversationsResponse,
        handlers::PaginatedMessagesResponse,
        handlers::PaginationMetaResponse,
        handlers::ConversationWithMessagesResponse,
        handlers::ShareResponse,
        handlers::CreateConversationApiRequest,
        handlers::UpdateConversationApiRequest,
        handlers::CreateMessageApiRequest,
        handlers::UpdateMessageApiRequest,
        handlers::CreateFolderApiRequest,
        handlers::UpdateFolderApiRequest,
        handlers::BulkOperationRequest,
        handlers::BulkOperationResponse,
        handlers::BulkArchiveRequest,
        handlers::BulkMoveRequest,
        handlers::ImportConversationsRequest,
        handlers::ImportConversationsResponse,
        handlers::ImportErrorResponse,
        handlers::ListConversationsParams,
        handlers::ListMessagesParams,
        // Pipeline schemas (SDK-aligned)
        handlers::EnhancedPipelineStatusResponse,
        handlers::PipelineMessageResponse,
        handlers::CancelPipelineResponse,
        handlers::QueueMetricsResponse,
        // Tasks schemas (SDK-aligned)
        handlers::ListTasksQuery,
        handlers::TaskResponse,
        handlers::TaskErrorResponse,
        handlers::TaskListResponse,
        handlers::PaginationInfo,
        handlers::StatisticsInfo,
        // Costs schemas (SDK-aligned)
        handlers::ModelPricingResponse,
        handlers::CostSummaryResponse,
        handlers::EstimateCostRequest,
        handlers::EstimateCostResponse,
        handlers::CostHistoryQuery,
        handlers::OperationCostResponse,
        handlers::AvailablePricingResponse,
        handlers::WorkspaceCostSummaryResponse,
        handlers::OperationBreakdown,
        handlers::BudgetInfo,
        handlers::CostHistoryPoint,
        // Workspaces & Tenants schemas (SDK-aligned)
        handlers::CreateTenantRequest,
        handlers::UpdateTenantRequest,
        handlers::TenantResponse,
        handlers::TenantListResponse,
        handlers::CreateWorkspaceApiRequest,
        handlers::UpdateWorkspaceApiRequest,
        handlers::WorkspaceResponse,
        handlers::WorkspaceListResponse,
        handlers::PaginationParams,
        handlers::WorkspaceStatsResponse,
        handlers::MetricsSnapshotDTO,
        handlers::MetricsHistoryResponse,
        handlers::RebuildEmbeddingsRequest,
        handlers::RebuildEmbeddingsResponse,
        handlers::ReprocessAllRequest,
        handlers::ReprocessAllResponse,
        handlers::RebuildKnowledgeGraphRequest,
        handlers::RebuildKnowledgeGraphResponse,
        // Lineage & Provenance schemas (OODA-28)
        handlers::ChunkDetailResponse,
        handlers::EntityProvenanceResponse,
        handlers::EntityLineageResponse,
        handlers::ChunkLineageResponse,
        handlers::DocumentGraphLineageResponse,
        handlers::ExtractionStatsResponse,
        handlers::LineRangeInfo,
        handlers::ChunkSourceInfo,
        handlers::EntitySourceInfo,
        handlers::EntitySummaryResponse,
        handlers::RelationshipSummaryResponse,
        handlers::RelatedEntityInfo,
        handlers::SourceDocumentInfo,
        handlers::ExtractedEntityInfo,
        handlers::ExtractedRelationshipInfo,
        handlers::ExtractionMetadataInfo,
        handlers::DescriptionVersionResponse,
        handlers::ExportParams,
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
        (name = "Chat", description = "Chat completion endpoints for conversational RAG queries"),
        (name = "Conversations", description = "Conversation management - list, create, update, delete conversations and messages"),
        (name = "Folders", description = "Folder management for organizing conversations"),
        (name = "Pipeline", description = "Document processing pipeline status and management"),
        (name = "Tasks", description = "Background task tracking and management"),
        (name = "Costs", description = "LLM usage cost tracking and estimation"),
        (name = "Tenants", description = "Multi-tenant organization management"),
        (name = "Workspaces", description = "Workspace management within tenants"),
        (name = "Lineage", description = "Data lineage and provenance tracking"),
        (name = "PDF", description = "PDF document upload and processing"),
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
