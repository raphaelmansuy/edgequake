//! EdgeQuake REST API Routes
//!
//! This module defines the complete HTTP routing configuration for the EdgeQuake API.
//!
//! ## Implements
//!
//! - [`FEAT0450`]: RESTful API routing
//! - [`FEAT0451`]: Versioned API endpoints
//! - [`FEAT0452`]: Health and readiness probes
//! - [`FEAT0453`]: WebSocket endpoints
//!
//! ## Use Cases
//!
//! - [`UC2050`]: Client accesses versioned API
//! - [`UC2051`]: Kubernetes checks health/readiness
//! - [`UC2052`]: Client receives real-time updates via WebSocket
//!
//! ## Enforces
//!
//! - [`BR0450`]: All business endpoints under /api/v1/
//! - [`BR0451`]: Consistent route naming conventions
//!
//! # API Design Principles
//!
//! - **RESTful**: Resources are nouns, HTTP verbs indicate actions
//! - **Versioned**: All business endpoints under `/api/v1/`
//! - **Consistent**: Uniform error responses (see [`crate::error`])
//! - **Documented**: OpenAPI 3.0 spec at `/swagger-ui/` (when enabled)
//!
//! # Route Structure
//!
//! ```text
//! /                           # Root
//! ├── health                  # Health check (GET)
//! ├── ready                   # Kubernetes readiness probe (GET)
//! ├── live                    # Kubernetes liveness probe (GET)
//! ├── metrics                 # Prometheus metrics (GET)
//! ├── ws/                     # WebSocket endpoints
//! │   └── pipeline/progress   # Real-time pipeline updates
//! ├── api/                    # Ollama-compatible API
//! │   ├── version            # GET - Ollama version
//! │   ├── tags               # GET - List available models
//! │   ├── ps                 # GET - Running model processes
//! │   ├── generate           # POST - Text generation
//! │   └── chat               # POST - Chat completion
//! └── api/v1/                 # Versioned API
//!     ├── auth/              # Authentication
//!     ├── users/             # User management
//!     ├── api-keys/          # API key management
//!     ├── tenants/           # Multi-tenant management
//!     ├── workspaces/        # Workspace management
//!     ├── documents/         # Document ingestion
//!     ├── query/             # RAG queries
//!     ├── chat/              # Chat completions
//!     ├── conversations/     # Conversation history
//!     ├── folders/           # Conversation organization
//!     ├── shared/            # Public conversation access
//!     └── graph/             # Knowledge graph operations
//! ```
//!
//! # HTTP Methods
//!
//! | Method   | Purpose                   | Idempotent | Safe |
//! |----------|---------------------------|------------|------|
//! | `GET`    | Retrieve resource(s)      | Yes        | Yes  |
//! | `POST`   | Create resource or action | No         | No   |
//! | `PUT`    | Replace resource          | Yes        | No   |
//! | `PATCH`  | Partial update            | No         | No   |
//! | `DELETE` | Remove resource           | Yes        | No   |
//!
//! # Authentication
//!
//! Most endpoints require authentication via:
//! - `Authorization: Bearer <JWT>` - Obtained from `/api/v1/auth/login`
//! - `X-API-Key: <key>` - Created via `/api/v1/api-keys`
//!
//! # Multi-Tenancy
//!
//! Tenant context is automatically extracted from:
//! 1. JWT claims (`tenant_id`, `workspace_id`)
//! 2. Headers (`X-Tenant-ID`, `X-Workspace-ID`)
//! 3. Default tenant (for non-authenticated deployments)

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

use crate::handlers;
use crate::state::AppState;

/// Create the API router.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check))
        .route("/live", get(handlers::liveness_check))
        // Metrics endpoint (Phase 3)
        .route("/metrics", get(handlers::get_metrics))
        // WebSocket endpoints (Phase 5)
        .route("/ws/pipeline/progress", get(handlers::ws_pipeline_progress))
        // OODA-15: Filtered WebSocket for specific PDF upload progress
        .route(
            "/ws/progress/{track_id}",
            get(handlers::ws_progress_by_track_id),
        )
        // Ollama Emulation API (GAP-038)
        .nest("/api", ollama_api_routes())
        // API v1 endpoints
        .nest("/api/v1", api_v1_routes())
        .with_state(state)
}

/// Ollama-compatible API routes.
///
/// These routes emulate the Ollama API, allowing EdgeQuake to be used
/// as a drop-in replacement for Ollama with tools like OpenWebUI.
fn ollama_api_routes() -> Router<AppState> {
    Router::new()
        .route("/version", get(handlers::ollama_version))
        .route("/tags", get(handlers::ollama_tags))
        .route("/ps", get(handlers::ollama_ps))
        .route("/generate", post(handlers::ollama_generate))
        .route("/chat", post(handlers::ollama_chat))
}

/// API v1 routes.
fn api_v1_routes() -> Router<AppState> {
    Router::new()
        // Authentication (Phase 3)
        .route("/auth/login", post(handlers::login))
        .route("/auth/refresh", post(handlers::refresh_token))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/me", get(handlers::get_me))
        // Users (Phase 3)
        .route("/users", post(handlers::create_user))
        .route("/users", get(handlers::list_users))
        .route("/users/{user_id}", get(handlers::get_user))
        .route("/users/{user_id}", delete(handlers::delete_user))
        // API Keys (Phase 3)
        .route("/api-keys", post(handlers::create_api_key))
        .route("/api-keys", get(handlers::list_api_keys))
        .route("/api-keys/{key_id}", delete(handlers::revoke_api_key))
        // Tenants (Multi-tenancy)
        .route("/tenants", post(handlers::create_tenant))
        .route("/tenants", get(handlers::list_tenants))
        .route("/tenants/{tenant_id}", get(handlers::get_tenant))
        .route("/tenants/{tenant_id}", put(handlers::update_tenant))
        .route("/tenants/{tenant_id}", delete(handlers::delete_tenant))
        // Workspaces (Multi-tenancy)
        .route(
            "/tenants/{tenant_id}/workspaces",
            post(handlers::create_workspace),
        )
        .route(
            "/tenants/{tenant_id}/workspaces",
            get(handlers::list_workspaces),
        )
        // Get workspace by slug (for URL-based routing)
        .route(
            "/tenants/{tenant_id}/workspaces/by-slug/{slug}",
            get(handlers::get_workspace_by_slug),
        )
        .route("/workspaces/{workspace_id}", get(handlers::get_workspace))
        .route(
            "/workspaces/{workspace_id}",
            put(handlers::update_workspace),
        )
        .route(
            "/workspaces/{workspace_id}",
            delete(handlers::delete_workspace),
        )
        .route(
            "/workspaces/{workspace_id}/stats",
            get(handlers::get_workspace_stats),
        )
        // OODA-22: Metrics history for workspace
        .route(
            "/workspaces/{workspace_id}/metrics-history",
            get(handlers::get_metrics_history),
        )
        // OODA-26: Manual metrics snapshot trigger
        .route(
            "/workspaces/{workspace_id}/metrics-snapshot",
            post(handlers::trigger_metrics_snapshot),
        )
        // SPEC-032: Rebuild embeddings for workspace
        .route(
            "/workspaces/{workspace_id}/rebuild-embeddings",
            post(handlers::rebuild_embeddings),
        )
        // Rebuild knowledge graph (LLM model change)
        .route(
            "/workspaces/{workspace_id}/rebuild-knowledge-graph",
            post(handlers::rebuild_knowledge_graph),
        )
        // SPEC-032: Reprocess all documents for workspace (Focus Area 5)
        .route(
            "/workspaces/{workspace_id}/reprocess-documents",
            post(handlers::reprocess_all_documents),
        )
        // Documents
        .route("/documents", post(handlers::upload_document))
        .route("/documents", get(handlers::list_documents))
        .route("/documents", delete(handlers::delete_all_documents))
        // Track Status (Phase 2) - MUST come before /documents/{document_id}
        .route(
            "/documents/track/{track_id}",
            get(handlers::get_track_status),
        )
        // File Upload (multipart) - MUST come before /documents/{document_id}
        .route("/documents/upload", post(handlers::upload_file))
        .route(
            "/documents/upload/batch",
            post(handlers::upload_files_batch),
        )
        // PDF Upload (SPEC-007) - MUST come before /documents/{document_id}
        .route("/documents/pdf", post(handlers::upload_pdf_document))
        .route("/documents/pdf", get(handlers::list_pdfs))
        // OODA-14: PDF progress endpoint - before /documents/pdf/{pdf_id}
        .route(
            "/documents/pdf/progress/{track_id}",
            get(handlers::get_pdf_progress),
        )
        // OODA-17: Error recovery endpoints - before /documents/pdf/{pdf_id}
        .route(
            "/documents/pdf/{pdf_id}/retry",
            post(handlers::retry_pdf_processing),
        )
        .route(
            "/documents/pdf/{pdf_id}/cancel",
            delete(handlers::cancel_pdf_processing),
        )
        // SPEC-002: PDF content download/view endpoints - before /documents/pdf/{pdf_id}
        .route(
            "/documents/pdf/{pdf_id}/download",
            get(handlers::download_pdf),
        )
        .route(
            "/documents/pdf/{pdf_id}/content",
            get(handlers::get_pdf_content),
        )
        .route("/documents/pdf/{pdf_id}", get(handlers::get_pdf_status))
        .route("/documents/pdf/{pdf_id}", delete(handlers::delete_pdf))
        // Document Scan API (GAP-014) - MUST come before /documents/{document_id}
        .route("/documents/scan", post(handlers::scan_directory))
        // Reprocess Failed Documents (GAP-039) - MUST come before /documents/{document_id}
        .route("/documents/reprocess", post(handlers::reprocess_failed))
        // Recover Stuck Processing Documents - MUST come before /documents/{document_id}
        .route("/documents/recover-stuck", post(handlers::recover_stuck))
        // Document deletion impact analysis - MUST come before /documents/{document_id}
        .route(
            "/documents/{document_id}/deletion-impact",
            get(handlers::analyze_deletion_impact),
        )
        // OODA-03: Chunk-level retry endpoints - MUST come before /documents/{document_id}
        .route(
            "/documents/{document_id}/retry-chunks",
            post(handlers::retry_failed_chunks),
        )
        .route(
            "/documents/{document_id}/failed-chunks",
            get(handlers::list_failed_chunks),
        )
        // OODA-07: Full lineage and metadata endpoints
        .route(
            "/documents/{document_id}/lineage",
            get(handlers::get_document_full_lineage),
        )
        .route(
            "/documents/{document_id}/metadata",
            get(handlers::get_document_metadata),
        )
        // OODA-22: Lineage export endpoint (JSON/CSV download)
        .route(
            "/documents/{document_id}/lineage/export",
            get(handlers::export_document_lineage),
        )
        // Document by ID - comes last because {document_id} matches any path segment
        .route("/documents/{document_id}", get(handlers::get_document))
        .route(
            "/documents/{document_id}",
            delete(handlers::delete_document),
        )
        // Query
        .route("/query", post(handlers::execute_query))
        .route("/query/stream", post(handlers::stream_query))
        // Chat (Unified chat completions API - preferred for client applications)
        .route("/chat/completions", post(handlers::chat_completion))
        .route(
            "/chat/completions/stream",
            post(handlers::chat_completion_stream),
        )
        // Conversations
        .route("/conversations", get(handlers::list_conversations))
        .route("/conversations", post(handlers::create_conversation))
        .route(
            "/conversations/import",
            post(handlers::import_conversations),
        )
        .route(
            "/conversations/bulk/delete",
            post(handlers::bulk_delete_conversations),
        )
        .route(
            "/conversations/bulk/archive",
            post(handlers::bulk_archive_conversations),
        )
        .route(
            "/conversations/bulk/move",
            post(handlers::bulk_move_conversations),
        )
        .route("/conversations/{id}", get(handlers::get_conversation))
        .route("/conversations/{id}", patch(handlers::update_conversation))
        .route("/conversations/{id}", delete(handlers::delete_conversation))
        .route("/conversations/{id}/messages", get(handlers::list_messages))
        .route(
            "/conversations/{id}/messages",
            post(handlers::create_message),
        )
        .route(
            "/conversations/{id}/share",
            post(handlers::share_conversation),
        )
        .route(
            "/conversations/{id}/share",
            delete(handlers::unshare_conversation),
        )
        // Messages
        .route("/messages/{message_id}", patch(handlers::update_message))
        .route("/messages/{message_id}", delete(handlers::delete_message))
        // Folders
        .route("/folders", get(handlers::list_folders))
        .route("/folders", post(handlers::create_folder))
        .route("/folders/{folder_id}", patch(handlers::update_folder))
        .route("/folders/{folder_id}", delete(handlers::delete_folder))
        // Shared conversations (public access)
        .route("/shared/{share_id}", get(handlers::get_shared_conversation))
        // Graph
        .route("/graph", get(handlers::get_graph))
        .route("/graph/stream", get(handlers::stream_graph))
        .route("/graph/nodes/{node_id}", get(handlers::get_node))
        .route("/graph/nodes/search", get(handlers::search_nodes))
        .route("/graph/labels/search", get(handlers::search_labels))
        .route("/graph/labels/popular", get(handlers::get_popular_labels))
        .route("/graph/degrees/batch", post(handlers::get_degrees_batch))
        // Entities (Phase 2)
        .route(
            "/graph/entities",
            get(handlers::list_entities).post(handlers::create_entity),
        )
        .route("/graph/entities/exists", get(handlers::entity_exists))
        .route("/graph/entities/merge", post(handlers::merge_entities))
        .route("/graph/entities/{entity_name}", get(handlers::get_entity))
        .route(
            "/graph/entities/{entity_name}",
            put(handlers::update_entity),
        )
        .route(
            "/graph/entities/{entity_name}",
            delete(handlers::delete_entity),
        )
        .route(
            "/graph/entities/{entity_name}/neighborhood",
            get(handlers::get_entity_neighborhood),
        )
        // Relationships (Phase 2)
        .route(
            "/graph/relationships",
            get(handlers::list_relationships).post(handlers::create_relationship),
        )
        .route(
            "/graph/relationships/{relationship_id}",
            get(handlers::get_relationship),
        )
        .route(
            "/graph/relationships/{relationship_id}",
            put(handlers::update_relationship),
        )
        .route(
            "/graph/relationships/{relationship_id}",
            delete(handlers::delete_relationship),
        )
        // Tasks
        .route("/tasks/{track_id}", get(handlers::get_task))
        .route("/tasks", get(handlers::list_tasks))
        .route("/tasks/{track_id}/cancel", post(handlers::cancel_task))
        .route("/tasks/{track_id}/retry", post(handlers::retry_task))
        // Pipeline (Phase 3)
        .route("/pipeline/status", get(handlers::get_pipeline_status))
        .route("/pipeline/cancel", post(handlers::cancel_pipeline))
        // OODA-20: Queue metrics for Objective B (Workspace-Level Task Queue Visibility)
        .route("/pipeline/queue-metrics", get(handlers::get_queue_metrics))
        // Cost Tracking (Phase 5)
        .route("/pipeline/costs/pricing", get(handlers::get_model_pricing))
        .route("/pipeline/costs/estimate", post(handlers::estimate_cost))
        // Cost Summary (WebUI Spec WEBUI-007)
        .route("/costs/summary", get(handlers::get_cost_summary))
        .route("/costs/history", get(handlers::get_cost_history))
        .route("/costs/budget", get(handlers::get_budget_status))
        .route("/costs/budget", patch(handlers::update_budget))
        // Lineage (Phase 5)
        .route(
            "/lineage/entities/{entity_name}",
            get(handlers::get_entity_lineage),
        )
        .route(
            "/lineage/documents/{document_id}",
            get(handlers::get_document_lineage),
        )
        // Chunk Detail (WebUI Spec WEBUI-006)
        .route("/chunks/{chunk_id}", get(handlers::get_chunk_detail))
        // OODA-08: Chunk lineage with parent refs
        .route(
            "/chunks/{chunk_id}/lineage",
            get(handlers::get_chunk_lineage),
        )
        // Entity Provenance (WebUI Spec WEBUI-006)
        .route(
            "/entities/{entity_id}/provenance",
            get(handlers::get_entity_provenance),
        )
        // Settings (Provider Status) (SPEC-032 Phase 5E)
        .route(
            "/settings/provider/status",
            get(handlers::get_provider_status),
        )
        // List available providers (SPEC-032 OODA 12)
        .route(
            "/settings/providers",
            get(handlers::list_available_providers),
        )
        // Models Configuration API (SPEC-032 OODA 66-70)
        .route("/models", get(handlers::list_models))
        .route("/models/llm", get(handlers::list_llm_models))
        .route("/models/embedding", get(handlers::list_embedding_models))
        .route("/models/health", get(handlers::check_providers_health))
        .route("/models/{provider}", get(handlers::get_provider))
        .route("/models/{provider}/{model}", get(handlers::get_model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_route() {
        let state = AppState::test_state();
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_route() {
        let state = AppState::test_state();
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
