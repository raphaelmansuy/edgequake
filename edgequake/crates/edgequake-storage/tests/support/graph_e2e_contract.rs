//! Shared graph storage E2E contract (STORE-DRY-003 / P2-11).
#![allow(dead_code)]

use std::collections::HashMap;

use edgequake_storage::traits::GraphStorage;

fn node_props(entity_type: &str) -> HashMap<String, serde_json::Value> {
    let mut props = HashMap::new();
    props.insert("entity_type".to_string(), serde_json::json!(entity_type));
    props
}

fn edge_props(rel_type: &str) -> HashMap<String, serde_json::Value> {
    let mut props = HashMap::new();
    props.insert("relationship_type".to_string(), serde_json::json!(rel_type));
    props
}

/// Node create-read-update-delete.
pub async fn assert_graph_node_crud<G: GraphStorage + ?Sized>(storage: &G) {
    storage
        .upsert_node("NODE_A", node_props("PERSON"))
        .await
        .unwrap();
    assert!(storage.has_node("NODE_A").await.unwrap());

    let node = storage.get_node("NODE_A").await.unwrap().unwrap();
    assert_eq!(node.id, "NODE_A");
    assert_eq!(node.properties["entity_type"], "PERSON");

    storage
        .upsert_node("NODE_A", node_props("ORGANIZATION"))
        .await
        .unwrap();
    assert_eq!(
        storage
            .get_node("NODE_A")
            .await
            .unwrap()
            .unwrap()
            .properties["entity_type"],
        "ORGANIZATION"
    );

    storage.delete_node("NODE_A").await.unwrap();
    assert!(!storage.has_node("NODE_A").await.unwrap());
}

/// Edge create-read-update-delete between two nodes.
pub async fn assert_graph_edge_crud<G: GraphStorage + ?Sized>(storage: &G) {
    storage
        .upsert_node("SOURCE", node_props("PERSON"))
        .await
        .unwrap();
    storage
        .upsert_node("TARGET", node_props("PERSON"))
        .await
        .unwrap();

    storage
        .upsert_edge("SOURCE", "TARGET", edge_props("KNOWS"))
        .await
        .unwrap();
    assert!(storage.has_edge("SOURCE", "TARGET").await.unwrap());

    storage
        .upsert_edge("SOURCE", "TARGET", edge_props("WORKS_WITH"))
        .await
        .unwrap();
    assert_eq!(
        storage
            .get_edge("SOURCE", "TARGET")
            .await
            .unwrap()
            .unwrap()
            .properties["relationship_type"],
        "WORKS_WITH"
    );

    storage.delete_edge("SOURCE", "TARGET").await.unwrap();
    assert!(!storage.has_edge("SOURCE", "TARGET").await.unwrap());
}

/// Hub-and-spoke pattern: degree and edge listing.
pub async fn assert_graph_node_edges<G: GraphStorage + ?Sized>(storage: &G) {
    storage.upsert_node("HUB", node_props("ORG")).await.unwrap();
    for i in 0..5 {
        let id = format!("SPOKE_{i}");
        storage
            .upsert_node(&id, node_props("PERSON"))
            .await
            .unwrap();
        storage
            .upsert_edge("HUB", &id, edge_props("LINKS"))
            .await
            .unwrap();
    }

    assert_eq!(storage.get_node_edges("HUB").await.unwrap().len(), 5);
    assert_eq!(storage.node_degree("HUB").await.unwrap(), 5);
}
