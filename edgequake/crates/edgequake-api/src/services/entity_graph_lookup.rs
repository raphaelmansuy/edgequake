//! Entity graph node lookup SSOT (SPEC-027 IMP-026 ascending-compat).
//!
//! Resolves user-supplied identifiers to graph nodes: tries normalized entity
//! name first, then the raw path segment (supports legacy/raw node keys).

use edgequake_storage::traits::GraphStorageReadOps;
use edgequake_storage::GraphNode;

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::properties_match_tenant_context;
use crate::middleware::TenantContext;
use crate::services::entity_name_normalize::normalize_entity_name;

/// Ordered graph keys to attempt for a path/query entity identifier.
pub fn entity_lookup_candidates(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    let normalized = normalize_entity_name(trimmed);
    if normalized == trimmed {
        vec![normalized]
    } else {
        vec![normalized, trimmed.to_string()]
    }
}

/// Load a graph node by user-supplied id/name with tenant isolation (404 on cross-tenant).
pub async fn lookup_entity_node_for_context(
    graph: &dyn GraphStorageReadOps,
    raw_entity_id: &str,
    ctx: &TenantContext,
) -> ApiResult<GraphNode> {
    let candidates = entity_lookup_candidates(raw_entity_id);
    if candidates.is_empty() {
        return Err(ApiError::BadRequest("Entity identifier required".into()));
    }

    for key in &candidates {
        let Some(node) = graph.get_node(key).await? else {
            continue;
        };
        if properties_match_tenant_context(&node.properties, ctx) {
            return Ok(node);
        }
        return Err(ApiError::NotFound(format!(
            "Entity '{}' not found",
            raw_entity_id
        )));
    }

    let normalized = normalize_entity_name(raw_entity_id.trim());
    Err(ApiError::NotFound(format!(
        "Entity '{}' not found (normalized: '{}'). \
         Accepts normalized entity names (UPPERCASE_WITH_UNDERSCORES) or raw graph node ids.",
        raw_entity_id, normalized
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_normalize_first_then_raw() {
        let c = entity_lookup_candidates("Machine Learning");
        assert_eq!(c, vec!["MACHINE_LEARNING", "Machine Learning"]);
    }

    #[test]
    fn candidates_single_when_already_normalized() {
        let c = entity_lookup_candidates("MACHINE_LEARNING");
        assert_eq!(c, vec!["MACHINE_LEARNING"]);
    }
}
