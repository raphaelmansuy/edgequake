//! Guarded community detection — SPEC-006 P3 (SRP).
//!
//! Wraps `detect_communities` with `ResourceGuard` admission so full-graph
//! Louvain cannot run on monster workspaces without explicit rejection.

use edgequake_core::{AdmissionDecision, GraphOperation, ResourceGuard};
use edgequake_storage::community::detect_communities_unchecked;
use edgequake_storage::{CommunityConfig, CommunityDetectionResult, GraphStorage};
use std::sync::Arc;

use crate::error::{ApiError, ApiResult};

/// Detect communities after pre-flight graph size admission check.
pub async fn detect_communities_guarded(
    graph_storage: &Arc<dyn GraphStorage>,
    config: &CommunityConfig,
    guard: &ResourceGuard,
) -> ApiResult<CommunityDetectionResult> {
    let node_count = graph_storage
        .node_count_fast()
        .await
        .map_err(ApiError::from)?;

    match guard.admit_graph_operation(GraphOperation::CommunityDetection, node_count) {
        AdmissionDecision::Allow => detect_communities_unchecked(graph_storage, config)
            .await
            .map_err(ApiError::from),
        AdmissionDecision::RejectGraphTooLarge {
            node_count,
            threshold,
        } => Err(ApiError::graph_too_large(node_count, threshold)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::ResourceBudgetConfig;
    use edgequake_storage::adapters::memory::MemoryGraphStorage;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn rejects_community_detection_when_graph_exceeds_threshold() {
        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("community-guard"));
        for i in 0..50 {
            let mut props = HashMap::new();
            props.insert("tenant_id".to_string(), json!("t"));
            props.insert("workspace_id".to_string(), json!("w"));
            graph
                .upsert_node(&format!("N_{:03}", i), props)
                .await
                .unwrap();
        }

        let guard = ResourceGuard::new(ResourceBudgetConfig {
            graph_scan_threshold_nodes: 10,
            ..Default::default()
        });

        let result = detect_communities_guarded(&graph, &CommunityConfig::default(), &guard).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("too large") || err.contains("unavailable"));
    }
}
