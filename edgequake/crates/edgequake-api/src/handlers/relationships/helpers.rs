//! Shared helper functions for relationship handlers.
//!
//! Normalization, type extraction, and edge-to-response conversion.

use edgequake_storage::GraphEdge;

use crate::handlers::relationships_types::RelationshipResponse;

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
