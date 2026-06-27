//! Global query community expansion (SPEC-023 I6, SPEC-025 6.3).
//!
//! Uses index-time `community_id` labels — no Louvain at query time.
//! Resolves co-community nodes via push-down `list_nodes_filtered`, not popular scan.

use std::collections::HashSet;

use edgequake_storage::community_persist::community_features_enabled;
use edgequake_storage::traits::{GraphReadView, NodeListFilter};

use crate::context::QueryContext;
use crate::engine_impl::QueryEngineConfig;
use crate::error::Result;
use crate::helpers::build_entity_from_node;

/// Expand global context with co-community entities (same `community_id` property).
pub async fn expand_global_context_with_communities(
    config: &QueryEngineConfig,
    context: &mut QueryContext,
    entity_ids: &mut Vec<String>,
    graph: GraphReadView<'_>,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
) -> Result<()> {
    if !config.enable_community_global || !community_features_enabled() || entity_ids.is_empty() {
        return Ok(());
    }

    let nodes_map = graph.get_nodes_batch(entity_ids).await?;
    let seed_communities: HashSet<u64> = nodes_map
        .values()
        .filter_map(|n| n.properties.get("community_id").and_then(|v| v.as_u64()))
        .collect();

    if seed_communities.is_empty() {
        return Ok(());
    }

    let filter = NodeListFilter {
        tenant_id,
        workspace_id,
        community_ids: Some(seed_communities.into_iter().collect()),
        ..Default::default()
    };

    let limit = config.max_entities.saturating_mul(2).max(1);
    let page = graph.list_nodes_filtered(&filter, 0, limit).await?;

    let page_ids: Vec<String> = page.items.iter().map(|n| n.id.clone()).collect();
    let degrees: std::collections::HashMap<String, usize> = graph
        .node_degrees_batch(&page_ids)
        .await?
        .into_iter()
        .collect();

    let mut seen: HashSet<String> = context.entities.iter().map(|e| e.name.clone()).collect();

    for node in page.items {
        if !seen.insert(node.id.clone()) {
            continue;
        }
        let degree = degrees.get(&node.id).copied().unwrap_or(0);
        let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.5);
        context.add_entity(entity);
        if !entity_ids.contains(&node.id) {
            entity_ids.push(node.id.clone());
        }
        if context.entities.len() >= config.max_entities {
            break;
        }
    }

    Ok(())
}
