//! Multi-hop graph edge expansion for local/global query modes (SPEC-024 2.7).

use std::collections::HashSet;

use edgequake_storage::traits::{GraphEdge, GraphReadView};

/// BFS expansion of graph edges up to `depth` hops from seed node IDs.
///
/// Uses batched incident-edge lookup per BFS frontier (SPEC-025 6.2) instead of
/// N+1 `get_node_edges` calls per node.
///
/// `depth` 1 returns direct edges only. `depth` 0 returns an empty list.
pub async fn edges_within_depth(
    graph: &GraphReadView<'_>,
    seed_ids: &[String],
    depth: usize,
    max_edges: usize,
) -> edgequake_storage::error::Result<Vec<GraphEdge>> {
    if depth == 0 || seed_ids.is_empty() || max_edges == 0 {
        return Ok(Vec::new());
    }

    let mut frontier: Vec<String> = seed_ids.to_vec();
    let mut seen_nodes: HashSet<String> = seed_ids.iter().cloned().collect();
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();
    let mut collected = Vec::new();

    for _ in 0..depth {
        if frontier.is_empty() || collected.len() >= max_edges {
            break;
        }

        let batch = graph.get_incident_edges_batch(&frontier).await?;
        let mut next_frontier = Vec::new();

        for edge in batch {
            let key = (edge.source.clone(), edge.target.clone());
            if !seen_edges.insert(key) {
                continue;
            }
            let source = edge.source.clone();
            let target = edge.target.clone();
            collected.push(edge);
            if collected.len() >= max_edges {
                break;
            }

            for id in [source, target] {
                if seen_nodes.insert(id.clone()) {
                    next_frontier.push(id);
                }
            }
        }

        frontier = next_frontier;
    }

    collected.truncate(max_edges);
    Ok(collected)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use edgequake_storage::adapters::memory::MemoryGraphStorage;
    use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

    use super::*;

    #[tokio::test]
    async fn depth_one_returns_direct_edges_only() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        graph.upsert_edge("B", "C", HashMap::new()).await.unwrap();

        let view = GraphReadView::new(&graph);
        let edges = edges_within_depth(&view, &["A".to_string()], 1, 10)
            .await
            .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, "B");
    }

    #[tokio::test]
    async fn depth_two_reaches_second_hop() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        graph.upsert_edge("B", "C", HashMap::new()).await.unwrap();

        let view = GraphReadView::new(&graph);
        let edges = edges_within_depth(&view, &["A".to_string()], 2, 10)
            .await
            .unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    async fn batch_frontier_deduplicates_star_edges() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        for leaf in ["B", "C", "D"] {
            graph
                .upsert_edge("HUB", leaf, HashMap::new())
                .await
                .unwrap();
        }

        let view = GraphReadView::new(&graph);
        let edges = edges_within_depth(&view, &["HUB".to_string()], 1, 10)
            .await
            .unwrap();
        assert_eq!(edges.len(), 3);
    }
}
