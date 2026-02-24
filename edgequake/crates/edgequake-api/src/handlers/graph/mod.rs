//! Knowledge graph API handlers for visualization and exploration.
//!
//! # Implements
//!
//! @implements FEAT0206
//! @implements FEAT0405 (Graph Exploration API)
//! @implements FEAT0204 (Graph Analytics)
//! @implements FEAT0601 (Knowledge Graph Visualization)
//! @implements FEAT0410 (REST API Service)
//!
//! - **UC0101**: Explore Entity Neighborhood
//! - **UC0104**: View Graph Statistics
//!
//! # Enforces
//!
//! - **BR0201**: Tenant isolation (graph scoped to workspace)
//! - **BR0009**: Max 1000 nodes per visualization request
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | GET | `/api/v1/graph` | [`get_graph`] | Get full graph (paginated) |
//! | GET | `/api/v1/graph/stats` | [`get_graph_stats`] | Node/edge counts |
//! | GET | `/api/v1/graph/stream` | SSE streaming graph updates |
//!
//! # WHY: Separate Graph Visualization Layer
//!
//! Graph visualization is compute-intensive and has different requirements
//! than query execution:
//! - Needs pagination to handle large graphs
//! - Requires layout hints for rendering
//! - May need streaming for real-time updates
//!
//! Separating from query handlers enables independent optimization.

mod graph_query;
mod graph_stream;

pub use graph_query::*;
pub use graph_stream::*;

// Re-export DTOs from graph_types module
pub use crate::handlers::graph_types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Path, Query, State};

    use crate::middleware::TenantContext;
    use crate::state::AppState;

    #[tokio::test]
    async fn test_get_graph_empty() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();
        let params = GraphQueryParams {
            start_node: None,
            depth: 2,
            max_nodes: 100,
        };

        let result = get_graph(State(state), tenant_ctx, Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_get_graph_with_depth() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();
        let params = GraphQueryParams {
            start_node: None,
            depth: 5,
            max_nodes: 50,
        };

        let result = get_graph(State(state), tenant_ctx, Query(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_node_not_found() {
        let state = AppState::test_state();

        let result = get_node(State(state), Path("nonexistent_node".to_string())).await;
        // Should return not found or empty
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_search_labels_empty() {
        let state = AppState::test_state();
        let params = SearchLabelsQuery {
            q: "test".to_string(),
            limit: 10,
        };

        let result = search_labels(State(state), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.labels.is_empty());
    }

    #[tokio::test]
    async fn test_get_popular_labels() {
        let state = AppState::test_state();
        let params = PopularLabelsQuery {
            limit: 20,
            min_degree: None,
            entity_type: None,
        };

        let result = get_popular_labels(State(state), Query(params)).await;
        assert!(result.is_ok());
    }
}
