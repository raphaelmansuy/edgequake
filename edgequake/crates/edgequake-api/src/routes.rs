//! API routes.

use axum::{
    routing::{delete, get, post, put},
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
        // API v1 endpoints
        .nest("/api/v1", api_v1_routes())
        .with_state(state)
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
        // Documents
        .route("/documents", post(handlers::upload_document))
        .route("/documents", get(handlers::list_documents))
        .route("/documents/{document_id}", get(handlers::get_document))
        .route(
            "/documents/{document_id}",
            delete(handlers::delete_document),
        )
        // File Upload (multipart)
        .route("/documents/upload", post(handlers::upload_file))
        .route(
            "/documents/upload/batch",
            post(handlers::upload_files_batch),
        )
        // Query
        .route("/query", post(handlers::execute_query))
        .route("/query/stream", post(handlers::stream_query))
        // Graph
        .route("/graph", get(handlers::get_graph))
        .route("/graph/nodes/{node_id}", get(handlers::get_node))
        .route("/graph/labels/search", get(handlers::search_labels))
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
