//! Entity merge edge rewire — SPEC-027 cold-path batch I/O (SRP/DRY).
//!
//! Batches edge existence reads (`get_edges_for_node_set`) and writes (`upsert_edges_batch`).

use std::collections::{HashMap, HashSet};

use edgequake_storage::traits::GraphStorage;
use edgequake_storage::GraphEdge;

use crate::error::ApiResult;
use crate::handlers::isolation::stamp_tenant_context_properties;
use crate::middleware::TenantContext;

/// Outcome counters for merge edge rewire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeRewireStats {
    pub relationships_merged: usize,
    pub duplicate_relationships_removed: usize,
}

struct RewirePlan {
    new_source: String,
    new_target: String,
    incoming: std::collections::HashMap<String, serde_json::Value>,
}

/// Rewire source entity edges onto target using batched graph reads/writes.
pub async fn rewire_merged_entity_edges(
    graph: &dyn GraphStorage,
    source_edges: &[GraphEdge],
    source_entity: &str,
    target_entity: &str,
    tenant_ctx: &TenantContext,
) -> ApiResult<MergeRewireStats> {
    let tenant_id = tenant_ctx
        .tenant_id
        .as_deref()
        .ok_or_else(|| crate::error::ApiError::BadRequest("Tenant context required".into()))?;
    let workspace_id = tenant_ctx
        .workspace_id
        .as_deref()
        .ok_or_else(|| crate::error::ApiError::BadRequest("Workspace context required".into()))?;

    let mut plans = Vec::with_capacity(source_edges.len());
    let mut node_ids: HashSet<String> = HashSet::new();
    let mut stats = MergeRewireStats::default();
    node_ids.insert(target_entity.to_string());

    for edge in source_edges {
        let (new_source, new_target) = if edge.source == source_entity {
            (target_entity.to_string(), edge.target.clone())
        } else {
            (edge.source.clone(), target_entity.to_string())
        };

        if new_source == new_target {
            stats.duplicate_relationships_removed += 1;
            continue;
        }

        node_ids.insert(new_source.clone());
        node_ids.insert(new_target.clone());
        plans.push(RewirePlan {
            new_source,
            new_target,
            incoming: edge.properties.clone(),
        });
    }

    if plans.is_empty() {
        return Ok(stats);
    }

    let node_id_vec: Vec<String> = node_ids.into_iter().collect();
    let existing_edges = graph
        .get_edges_for_node_set(&node_id_vec, Some(tenant_id), Some(workspace_id))
        .await?;

    let mut existing_by_pair: HashMap<(String, String), GraphEdge> = HashMap::new();
    for edge in existing_edges {
        existing_by_pair.insert((edge.source.clone(), edge.target.clone()), edge);
    }

    let mut edges_to_upsert: Vec<(
        String,
        String,
        std::collections::HashMap<String, serde_json::Value>,
    )> = Vec::with_capacity(plans.len());

    for plan in plans {
        let pair = (plan.new_source.clone(), plan.new_target.clone());
        let existing_edge = existing_by_pair.get(&pair);
        if existing_edge.is_some() {
            stats.duplicate_relationships_removed += 1;
        }

        let mut merged_properties = merge_edge_properties(
            existing_edge.map(|edge| &edge.properties),
            &plan.incoming,
            source_entity,
        );
        stamp_tenant_context_properties(&mut merged_properties, tenant_ctx)?;

        edges_to_upsert.push((plan.new_source, plan.new_target, merged_properties));
        stats.relationships_merged += 1;
    }

    if !edges_to_upsert.is_empty() {
        graph.upsert_edges_batch(&edges_to_upsert).await?;
    }

    Ok(stats)
}

fn collect_string_values(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::String(s)) => vec![s.clone()],
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();

    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn collect_relation_terms(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::String(s)) => s
            .split([',', ';'])
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn relation_specificity_score(value: &str) -> usize {
    value.matches('_').count() * 10 + value.len()
}

fn select_primary_relation_type(values: &[String]) -> Option<String> {
    values
        .iter()
        .max_by_key(|value| relation_specificity_score(value))
        .cloned()
}

fn merge_edge_properties(
    existing: Option<&std::collections::HashMap<String, serde_json::Value>>,
    incoming: &std::collections::HashMap<String, serde_json::Value>,
    merged_from: &str,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut properties = existing.cloned().unwrap_or_default();

    for (key, value) in incoming {
        properties
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    let weight = existing
        .and_then(|props| props.get("weight"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
        .max(
            incoming
                .get("weight")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0),
        );

    if weight > 0.0 {
        properties.insert("weight".to_string(), serde_json::json!(weight));
    }

    let merged_from_values = dedupe_strings(
        collect_string_values(existing.and_then(|props| props.get("merged_from")))
            .into_iter()
            .chain(collect_string_values(incoming.get("merged_from")))
            .chain(std::iter::once(merged_from.to_string()))
            .collect(),
    );

    if !merged_from_values.is_empty() {
        properties.insert(
            "merged_from".to_string(),
            serde_json::json!(merged_from_values),
        );
    }

    let merged_relation_types = dedupe_strings(
        collect_relation_terms(existing.and_then(|props| props.get("merged_relation_types")))
            .into_iter()
            .chain(collect_relation_terms(
                existing.and_then(|props| props.get("relation_type")),
            ))
            .chain(collect_relation_terms(
                incoming.get("merged_relation_types"),
            ))
            .chain(collect_relation_terms(incoming.get("relation_type")))
            .collect(),
    );

    if let Some(primary_relation_type) = select_primary_relation_type(&merged_relation_types) {
        properties.insert(
            "relation_type".to_string(),
            serde_json::Value::String(primary_relation_type),
        );
    }

    if merged_relation_types.len() > 1 {
        properties.insert(
            "merged_relation_types".to_string(),
            serde_json::json!(merged_relation_types),
        );
    }

    let merged_keywords = dedupe_strings(
        collect_relation_terms(existing.and_then(|props| props.get("keywords")))
            .into_iter()
            .chain(collect_relation_terms(incoming.get("keywords")))
            .collect(),
    );

    if !merged_keywords.is_empty() {
        properties.insert(
            "keywords".to_string(),
            serde_json::Value::String(merged_keywords.join(", ")),
        );
    }

    let merged_descriptions = dedupe_strings(
        collect_string_values(existing.and_then(|props| props.get("description")))
            .into_iter()
            .chain(collect_string_values(incoming.get("description")))
            .collect(),
    );

    if !merged_descriptions.is_empty() {
        properties.insert(
            "description".to_string(),
            serde_json::Value::String(merged_descriptions.join(" / ")),
        );
    }

    properties
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_edge_properties_combines_weight_and_merged_from() {
        let mut incoming = std::collections::HashMap::new();
        incoming.insert("weight".to_string(), serde_json::json!(2.0));
        incoming.insert("relation_type".to_string(), serde_json::json!("WORKS_AT"));

        let merged = merge_edge_properties(None, &incoming, "SOURCE_A");
        assert_eq!(merged.get("weight").and_then(|v| v.as_f64()), Some(2.0));
        assert!(merged.get("merged_from").is_some());
    }
}
