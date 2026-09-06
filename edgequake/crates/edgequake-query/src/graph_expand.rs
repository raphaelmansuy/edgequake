//! Unified neighborhood expansion: BFS or Personalized PageRank (SPEC-046).
//!
//! Single entry point for local/global modes so walk strategy stays DRY.

use std::collections::HashSet;

use edgequake_storage::traits::{collect_source_references, GraphEdge, GraphReadView};

use crate::graph_hops::edges_within_depth;
use crate::graph_ppr::{
    adjacency_from_edges, personalized_pagerank, rank_edges_by_ppr, GraphWalkMode, PprConfig,
};
use crate::helpers::extract_document_id;
use crate::lineage_scope::lineage_intersects_allowed;

/// Keep edge iff provenance ∩ allow-set is non-empty (SPEC-146 M3).
/// Missing provenance under an active allow-set → drop (fail-closed).
pub fn edge_authorized_by_allow_set(
    edge: &GraphEdge,
    allowed_document_ids: Option<&[String]>,
) -> bool {
    let Some(allowed) = allowed_document_ids else {
        return true;
    };
    let id_set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
    let refs = collect_source_references(&edge.properties);
    if refs.is_empty() {
        return false;
    }
    let mut docs: Vec<String> = Vec::new();
    for r in &refs {
        if let Some(doc) = extract_document_id(r) {
            docs.push(doc);
        } else if r.len() == 36 {
            // Bare document UUID
            docs.push(r.clone());
        }
    }
    lineage_intersects_allowed(&docs, &id_set)
}

fn filter_edges_by_allow(
    edges: Vec<GraphEdge>,
    allowed_document_ids: Option<&[String]>,
) -> Vec<GraphEdge> {
    if allowed_document_ids.is_none() {
        return edges;
    }
    edges
        .into_iter()
        .filter(|e| edge_authorized_by_allow_set(e, allowed_document_ids))
        .collect()
}

/// Expand edges from seed entity IDs using the configured walk mode.
///
/// - **Bfs**: classic hop expansion (`edges_within_depth`).
/// - **Ppr**: fetch a generous BFS envelope, then re-rank edges by PPR mass
///   on that subgraph (HippoRAG-inspired; dual-node chunk mapping happens later).
/// - When `EDGEQUAKE_RELATION_SELECT=lightrag`, bypasses walk and uses LightRAG
///   incident-edge + `(rank, weight)` sort (051).
/// - SPEC-146: when `allowed_document_ids` is `Some`, gate each edge by
///   `source_ids ∩ allow` (fail-closed if provenance missing).
pub async fn expand_neighborhood_edges(
    graph: &GraphReadView<'_>,
    seed_ids: &[String],
    depth: usize,
    max_edges: usize,
    walk: GraphWalkMode,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> edgequake_storage::error::Result<Vec<GraphEdge>> {
    expand_neighborhood_edges_scoped(
        graph,
        seed_ids,
        depth,
        max_edges,
        walk,
        tenant_id,
        workspace_id,
        None,
    )
    .await
}

/// Like [`expand_neighborhood_edges`] with an optional document allow-set (SPEC-146).
pub async fn expand_neighborhood_edges_scoped(
    graph: &GraphReadView<'_>,
    seed_ids: &[String],
    depth: usize,
    max_edges: usize,
    walk: GraphWalkMode,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
    allowed_document_ids: Option<&[String]>,
) -> edgequake_storage::error::Result<Vec<GraphEdge>> {
    if seed_ids.is_empty() || max_edges == 0 {
        return Ok(Vec::new());
    }
    // Active empty allow-set → no edges.
    if matches!(allowed_document_ids, Some(ids) if ids.is_empty()) {
        return Ok(Vec::new());
    }

    if crate::relation_select::RelationSelectMode::from_env()
        == crate::relation_select::RelationSelectMode::LightRag
    {
        let edges = crate::relation_select::select_edges_lightrag(
            graph,
            seed_ids,
            max_edges,
            tenant_id,
            workspace_id,
        )
        .await?;
        return Ok(filter_edges_by_allow(edges, allowed_document_ids));
    }

    let edges = match walk {
        GraphWalkMode::Bfs => {
            edges_within_depth(graph, seed_ids, depth, max_edges, tenant_id, workspace_id).await?
        }
        GraphWalkMode::Ppr => {
            // Envelope: deeper / wider than final max so PPR has room to flow
            let envelope_depth = depth.max(2);
            let envelope_cap = max_edges.saturating_mul(4).max(max_edges).min(2_000);
            let envelope = edges_within_depth(
                graph,
                seed_ids,
                envelope_depth,
                envelope_cap,
                tenant_id,
                workspace_id,
            )
            .await?;
            if envelope.is_empty() {
                return Ok(Vec::new());
            }
            // Gate envelope before PPR so mass cannot flow through denied edges.
            let envelope = filter_edges_by_allow(envelope, allowed_document_ids);
            if envelope.is_empty() {
                return Ok(Vec::new());
            }
            let adj = adjacency_from_edges(&envelope);
            let scores = personalized_pagerank(&adj, seed_ids, &PprConfig::from_env());
            return Ok(rank_edges_by_ppr(&envelope, &scores, max_edges));
        }
    };
    Ok(filter_edges_by_allow(edges, allowed_document_ids))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use edgequake_storage::adapters::memory::MemoryGraphStorage;
    use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

    use super::*;

    #[tokio::test]
    async fn ppr_walk_returns_seed_adjacent_edges() {
        let graph = MemoryGraphStorage::new("ppr-expand");
        graph.initialize().await.unwrap();
        graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        graph.upsert_edge("B", "C", HashMap::new()).await.unwrap();
        graph.upsert_edge("X", "Y", HashMap::new()).await.unwrap();

        let view = GraphReadView::new(&graph);
        let edges = expand_neighborhood_edges(
            &view,
            &["A".to_string()],
            2,
            10,
            GraphWalkMode::Ppr,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(!edges.is_empty());
        assert!(
            edges.iter().any(|e| e.source == "A" || e.target == "A"),
            "PPR should retain seed-adjacent edges"
        );
    }

    #[test]
    fn spec146_edge_provenance_filter_fail_closed() {
        let public = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let secret = "ffffffff-1111-2222-3333-444444444444";

        let edge = GraphEdge::with_properties(
            "SECRET",
            "PUBLIC",
            HashMap::from([(
                "source_ids".into(),
                serde_json::json!([secret, public]),
            )]),
        );

        // Only public allowed → edge kept (intersection non-empty).
        assert!(edge_authorized_by_allow_set(
            &edge,
            Some(&[public.to_string()])
        ));

        // Neither allowed → drop.
        assert!(!edge_authorized_by_allow_set(
            &edge,
            Some(&["99999999-9999-9999-9999-999999999999".into()])
        ));

        // Missing provenance under active allow-set → fail-closed drop.
        let bare = GraphEdge::new("A", "B");
        assert!(!edge_authorized_by_allow_set(
            &bare,
            Some(&[public.to_string()])
        ));

        // No allow-set (flag off) → keep.
        assert!(edge_authorized_by_allow_set(&bare, None));
    }
}
