//! Shared helper functions for relationship handlers.
//!
//! Normalization, type extraction, and edge-to-response conversion.

use edgequake_storage::traits::{EdgeListFilter, GraphStorage};
use edgequake_storage::GraphEdge;
use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::handlers::relationships_types::RelationshipResponse;
use crate::middleware::TenantContext;

/// Normalize entity name to UPPERCASE with underscores.
pub(super) fn normalize_entity_name(name: &str) -> String {
    name.to_uppercase().replace(' ', "_")
}

/// Extract relation type from keywords.
pub(super) fn extract_relation_type(keywords: &str) -> String {
    // Simple heuristic: use first keyword as relation type
    keywords
        .split(',')
        .next()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase().replace(' ', "_"))
        .unwrap_or_else(|| "RELATED_TO".to_string())
}

/// SPEC-006 P2: bounded relationship lookup (no get_all_nodes scan).
pub(super) async fn find_relationship_edge(
    graph_storage: &Arc<dyn GraphStorage>,
    tenant_ctx: &TenantContext,
    relationship_id: &str,
) -> ApiResult<GraphEdge> {
    let filter = EdgeListFilter {
        tenant_id: tenant_ctx.tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        relationship_type: None,
    };
    graph_storage
        .find_edge_by_relationship_id(&filter, relationship_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Relationship '{}' not found", relationship_id)))
}

/// Convert [`GraphEdge`] to [`RelationshipResponse`].
pub(super) fn edge_to_relationship_response(edge: GraphEdge, rel_id: &str) -> RelationshipResponse {
    let props = &edge.properties;

    RelationshipResponse {
        id: rel_id.to_string(),
        src_id: edge.source.clone(),
        tgt_id: edge.target.clone(),
        relation_type: props
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED_TO")
            .to_string(),
        keywords: props
            .get("keywords")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        weight: props.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.8),
        description: props
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        source_id: props
            .get("source_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        created_at: props
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at: props
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        metadata: props
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    }
}
