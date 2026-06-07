//! Shared graph workspace stat contract (SPEC-017 P0).
//!
//! Runs identical fixture assertions against memory (always) and postgres (when configured).

use std::collections::HashMap;

use edgequake_storage::traits::GraphStorage;
use uuid::Uuid;

pub fn props(map: &[(&str, &str)]) -> HashMap<String, serde_json::Value> {
    map.iter()
        .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
        .collect()
}

/// Seed two workspaces with nodes/edges and assert workspace-scoped dashboard stats.
pub async fn assert_workspace_graph_stats<G: GraphStorage + ?Sized>(storage: &G) {
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();

    storage
        .upsert_node(
            "ENTITY_A",
            props(&[
                ("entity_type", "person"),
                ("workspace_id", &ws_a.to_string()),
            ]),
        )
        .await
        .unwrap();
    storage
        .upsert_node(
            "ENTITY_B",
            props(&[("entity_type", "org"), ("workspace_id", &ws_a.to_string())]),
        )
        .await
        .unwrap();
    storage
        .upsert_node(
            "ENTITY_C",
            props(&[
                ("entity_type", "person"),
                ("workspace_id", &ws_b.to_string()),
            ]),
        )
        .await
        .unwrap();
    storage
        .upsert_edge(
            "ENTITY_A",
            "ENTITY_B",
            props(&[
                ("relation_type", "works_at"),
                ("workspace_id", &ws_a.to_string()),
            ]),
        )
        .await
        .unwrap();

    assert_eq!(storage.node_count().await.unwrap(), 3);
    assert_eq!(storage.node_count_by_workspace(&ws_a).await.unwrap(), 2);
    assert_eq!(storage.node_count_by_workspace(&ws_b).await.unwrap(), 1);
    assert_eq!(storage.edge_count_by_workspace(&ws_a).await.unwrap(), 1);
    assert_eq!(storage.edge_count_by_workspace(&ws_b).await.unwrap(), 0);
    assert_eq!(
        storage
            .distinct_node_type_count_by_workspace(&ws_a)
            .await
            .unwrap(),
        2
    );
}
