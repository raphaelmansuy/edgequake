//! API routes.

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
        // Documents
        .route("/documents", post(handlers::upload_document))
        .route("/documents", get(handlers::list_documents))
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
        .route("/graph/nodes/{node_id}", get(handlers::get_node))
        .route("/graph/labels/search", get(handlers::search_labels))
        .route("/graph/labels/popular", get(handlers::get_popular_labels))
        // Entities (Phase 2)
        .route("/graph/entities", post(handlers::create_entity))
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
        // Relationships (Phase 2)
        .route("/graph/relationships", post(handlers::create_relationship))
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
        // Cost Tracking (Phase 5)
        .route("/pipeline/costs/pricing", get(handlers::get_model_pricing))
        .route("/pipeline/costs/estimate", post(handlers::estimate_cost))
        // Cost Summary (WebUI Spec WEBUI-007)
        .route("/costs/summary", get(handlers::get_cost_summary))
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
        // Entity Provenance (WebUI Spec WEBUI-006)
        .route(
            "/entities/{entity_id}/provenance",
            get(handlers::get_entity_provenance),
        )
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
