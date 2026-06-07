//! Graph node/edge → query context conversions (SPEC-017 CORE-DRY-011).

use edgequake_storage::traits::{GraphEdge, GraphNode};

use crate::types::{ContextEntity, ContextRelationship};

/// Map a graph node to a [`ContextEntity`] using standard property keys.
pub fn graph_node_to_context_entity(
    node: &GraphNode,
    fallback_id: &str,
    score: f32,
) -> ContextEntity {
    ContextEntity {
        name: node
            .properties
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_id)
            .to_string(),
        entity_type: node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        description: node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        score,
    }
}

/// Map a graph edge to a [`ContextRelationship`] using standard property keys.
pub fn graph_edge_to_context_relationship(edge: &GraphEdge, score: f32) -> ContextRelationship {
    ContextRelationship {
        source: edge.source.clone(),
        target: edge.target.clone(),
        relation_type: edge
            .properties
            .get("relation_type")
            .or_else(|| edge.properties.get("keywords"))
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED")
            .to_string(),
        description: edge
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_node() -> GraphNode {
        let mut props = HashMap::new();
        props.insert("name".into(), serde_json::json!("ALICE"));
        props.insert("entity_type".into(), serde_json::json!("PERSON"));
        props.insert("description".into(), serde_json::json!("Engineer"));
        GraphNode::with_properties("alice-id", props)
    }

    #[test]
    fn node_mapping_uses_properties() {
        let entity = graph_node_to_context_entity(&sample_node(), "fallback", 0.9);
        assert_eq!(entity.name, "ALICE");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.description, "Engineer");
        assert!((entity.score - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn node_mapping_falls_back_to_id() {
        let node = GraphNode::new("NODE_X");
        let entity = graph_node_to_context_entity(&node, "NODE_X", 1.0);
        assert_eq!(entity.name, "NODE_X");
        assert_eq!(entity.entity_type, "UNKNOWN");
        assert_eq!(entity.description, "");
    }

    #[test]
    fn edge_mapping_uses_description_and_defaults() {
        let mut props = HashMap::new();
        props.insert("description".into(), serde_json::json!("works with"));
        let edge = GraphEdge::with_properties("a", "b", props);
        let rel = graph_edge_to_context_relationship(&edge, 1.0);
        assert_eq!(rel.source, "a");
        assert_eq!(rel.target, "b");
        assert_eq!(rel.relation_type, "RELATED");
        assert_eq!(rel.description, "works with");
    }
}
