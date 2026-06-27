//! Global query community expansion (SPEC-023 I6).
//!
//! Uses index-time `community_id` labels — no Louvain at query time.

use std::collections::HashSet;

use edgequake_storage::community_persist::community_features_enabled;
use edgequake_storage::traits::GraphReadView;

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

    let popular = graph
        .get_popular_nodes_with_degree(
            config.max_entities * 2,
            None,
            None,
            tenant_id.as_deref(),
            workspace_id.as_deref(),
        )
        .await?;

    let mut seen: HashSet<String> = context.entities.iter().map(|e| e.name.clone()).collect();

    for (node, degree) in popular {
        let Some(cid) = node.properties.get("community_id").and_then(|v| v.as_u64()) else {
            continue;
        };
        if !seed_communities.contains(&cid) {
            continue;
        }
        if !seen.insert(node.id.clone()) {
            continue;
        }
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
