//! Shared e2e test fixtures (STORE-DRY-003 / P2-11).
#![allow(dead_code)]

use std::collections::HashMap;

/// Random namespace for test isolation.
pub fn generate_namespace() -> String {
    format!(
        "test_{}",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]
    )
}

pub fn create_node_properties(
    entity_type: &str,
    description: &str,
) -> HashMap<String, serde_json::Value> {
    let mut props = HashMap::new();
    props.insert("entity_type".to_string(), serde_json::json!(entity_type));
    props.insert("description".to_string(), serde_json::json!(description));
    props
}

pub fn create_edge_properties(rel_type: &str, weight: f32) -> HashMap<String, serde_json::Value> {
    let mut props = HashMap::new();
    props.insert("relationship_type".to_string(), serde_json::json!(rel_type));
    props.insert("weight".to_string(), serde_json::json!(weight));
    props
}
