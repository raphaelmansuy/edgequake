//! API routes.

use axum::{
    routing::{delete, get, post},
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
        // API v1 endpoints
        .nest("/api/v1", api_v1_routes())
        .with_state(state)
}

/// API v1 routes.
fn api_v1_routes() -> Router<AppState> {
    Router::new()
        // Documents
        .route("/documents", post(handlers::upload_document))
        .route("/documents", get(handlers::list_documents))
        .route("/documents/{document_id}", get(handlers::get_document))
        .route(
            "/documents/{document_id}",
            delete(handlers::delete_document),
        )
        // Query
        .route("/query", post(handlers::execute_query))
        .route("/query/stream", post(handlers::stream_query))
        // Graph
        .route("/graph", get(handlers::get_graph))
        .route("/graph/nodes/{node_id}", get(handlers::get_node))
        .route("/graph/labels/search", get(handlers::search_labels))
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
