//! Persist community detection labels on graph nodes (SPEC-023 I6).
//!
//! Index-time community refresh keeps query-time global search O(batch) instead of
//! running Louvain on every request. Refreshes are **debounced** per workspace
//! via [`crate::community_index_service`] (SPEC-024 Phase 1.3 / 4.2).

use std::sync::Arc;

use serde_json::json;

use crate::community::{detect_communities_unchecked, CommunityConfig, CommunityDetectionResult};
use crate::error::Result;
use crate::traits::GraphStorage;

/// Whether community index refresh + global query expansion is enabled (default: true).
///
/// Set `EDGEQUAKE_COMMUNITY_GLOBAL=false` to disable (backward compatible).
pub fn community_features_enabled() -> bool {
    std::env::var("EDGEQUAKE_COMMUNITY_GLOBAL")
        .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "off"))
        .unwrap_or(true)
}

/// Detect communities and write `community_id` onto node properties.
pub async fn detect_and_persist_communities(
    graph: Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let result = detect_communities_unchecked(&graph, config).await?;
    persist_community_labels(&graph, &result).await?;
    Ok(result)
}

/// Write `community_id` for all nodes in the detection result (batch upsert).
pub async fn persist_community_labels(
    graph: &Arc<dyn GraphStorage>,
    result: &CommunityDetectionResult,
) -> Result<usize> {
    if result.node_to_community.is_empty() {
        return Ok(0);
    }

    let ids: Vec<String> = result.node_to_community.keys().cloned().collect();
    let existing = graph.get_nodes_batch(&ids).await?;
    let mut batch = Vec::with_capacity(existing.len());

    for (node_id, community_id) in &result.node_to_community {
        let Some(node) = existing.get(node_id) else {
            continue;
        };
        let mut props = node.properties.clone();
        props.insert("community_id".to_string(), json!(community_id));
        batch.push((node_id.clone(), props));
    }

    if batch.is_empty() {
        return Ok(0);
    }

    let count = batch.len();
    graph.upsert_nodes_batch(&batch).await?;
    Ok(count)
}

/// True when a sample of graph nodes lacks `community_id` (legacy graphs pre-044).
pub async fn needs_community_backfill(graph: &Arc<dyn GraphStorage>) -> Result<bool> {
    let sample = graph
        .get_popular_nodes_with_degree(8, None, None, None, None)
        .await?;
    if sample.is_empty() {
        return Ok(false);
    }
    Ok(sample
        .iter()
        .any(|(node, _)| !node.properties.contains_key("community_id")))
}

/// One-shot backfill for existing graphs (startup / migration 044).
pub async fn backfill_communities_if_needed(
    graph: Arc<dyn GraphStorage>,
) -> Result<Option<CommunityDetectionResult>> {
    if !community_features_enabled() {
        return Ok(None);
    }
    if !needs_community_backfill(&graph).await? {
        return Ok(None);
    }

    let node_count = graph.node_count_fast().await.unwrap_or(0);
    let threshold = std::env::var("EDGEQUAKE_COMMUNITY_BACKFILL_MAX_NODES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000);
    if node_count > threshold {
        tracing::warn!(
            node_count,
            threshold,
            "Skipping automatic community backfill — graph too large (set EDGEQUAKE_COMMUNITY_BACKFILL_MAX_NODES)"
        );
        return Ok(None);
    }

    tracing::info!(
        node_count,
        "Running automatic community backfill (migration 044)"
    );
    let result = detect_and_persist_communities(graph, &CommunityConfig::default()).await?;
    tracing::info!(
        communities = result.communities.len(),
        labeled_nodes = result.node_to_community.len(),
        "Community backfill complete"
    );
    Ok(Some(result))
}

/// Refresh community labels after ingest merge (non-fatal on failure).
pub async fn refresh_community_index(graph: Arc<dyn GraphStorage>) {
    if !community_features_enabled() {
        return;
    }
    match detect_and_persist_communities(graph, &CommunityConfig::default()).await {
        Ok(result) => tracing::debug!(
            communities = result.communities.len(),
            labeled_nodes = result.node_to_community.len(),
            "Community index refreshed after ingest"
        ),
        Err(e) => tracing::warn!(
            error = %e,
            "Community index refresh failed (non-fatal)"
        ),
    }
}

/// Background startup backfill for legacy graphs (migration 044 / postgres bootstrap).
pub fn spawn_community_backfill_if_needed(graph: Arc<dyn GraphStorage>) {
    tokio::spawn(async move {
        match backfill_communities_if_needed(graph).await {
            Ok(Some(_)) => tracing::info!("Automatic community backfill complete"),
            Ok(None) => {}
            Err(e) => tracing::warn!(
                error = %e,
                "Community backfill failed — will retry on next restart"
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;
    use std::collections::HashMap;

    #[tokio::test]
    async fn persist_community_labels_writes_property() {
        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("comm-persist"));
        graph.initialize().await.unwrap();
        graph.upsert_node("A", HashMap::new()).await.unwrap();
        graph.upsert_node("B", HashMap::new()).await.unwrap();
        graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();

        let mut result = CommunityDetectionResult::new();
        result.node_to_community.insert("A".to_string(), 0);
        result.node_to_community.insert("B".to_string(), 0);

        let n = persist_community_labels(&graph, &result).await.unwrap();
        assert_eq!(n, 2);

        let node = graph.get_node("A").await.unwrap().unwrap();
        assert_eq!(
            node.properties.get("community_id").and_then(|v| v.as_u64()),
            Some(0)
        );
    }
}
