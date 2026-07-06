//! SPEC-044 — saga graph compensation on live PostgreSQL + AGE.
//!
//! Proves `compensate_orphan_graph_writes` → `delete_node` uses fixed
//! `cypher_execute_bound` (bare `$1`), matching production merge-failure path.

#[path = "support/postgres_test_config.rs"]
#[cfg(feature = "postgres")]
mod postgres_test_config;

#[cfg(feature = "postgres")]
mod postgres_integration {
    use super::postgres_test_config;
    use std::collections::HashMap;

    use edgequake_storage::compensation;
    use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps};
    use edgequake_storage::PostgresAGEGraphStorage;

    #[tokio::test]
    async fn spec044_compensation_deletes_orphan_node_via_bound_cypher() {
        let Some(config) = postgres_test_config::contract_postgres_config("spec044_compensation")
        else {
            eprintln!("SKIP spec044_compensation: DATABASE_URL or POSTGRES_PASSWORD not set");
            return;
        };

        let storage = PostgresAGEGraphStorage::new(config);
        storage.initialize().await.expect("graph init");

        let node_id = "SPEC044_COMP_NODE";
        storage
            .upsert_node(
                node_id,
                HashMap::from([("entity_type".to_string(), serde_json::json!("PROBE"))]),
            )
            .await
            .expect("seed node");
        assert!(storage.has_node(node_id).await.unwrap());

        compensation::compensate_orphan_graph_writes(
            &storage,
            "spec044-doc",
            &[node_id.to_string()],
            &[],
            "merge failed (spec044 test)",
        )
        .await;

        assert!(
            !storage.has_node(node_id).await.unwrap(),
            "compensation must delete orphan node via parameterized delete_node"
        );
    }
}
