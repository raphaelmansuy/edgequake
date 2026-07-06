//! SPEC-022 P-H7 — parameterized AGE Cypher on hot-path node/edge CRUD.
//! SPEC-044 — source + integration gates for bare `$1` bind contract.

#[path = "support/postgres_test_config.rs"]
#[cfg(feature = "postgres")]
mod postgres_test_config;

#[cfg(feature = "postgres")]
mod postgres_integration {
    use super::postgres_test_config;
    use std::collections::HashMap;

    use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps};
    use edgequake_storage::PostgresAGEGraphStorage;

    #[tokio::test]
    async fn spec022_postgres_cypher_prepared_node_crud_injection_safe() {
        let Some(config) = postgres_test_config::contract_postgres_config("spec022_cypher") else {
            eprintln!(
                "SKIP spec022_postgres_cypher_prepared: DATABASE_URL or POSTGRES_PASSWORD not set"
            );
            return;
        };

        let storage = PostgresAGEGraphStorage::new(config);
        storage.initialize().await.expect("graph init");

        let node_id = "INJECT' OR 1=1 --";
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("PERSON"));
        props.insert(
            "description".to_string(),
            serde_json::json!("injection probe"),
        );

        storage.upsert_node(node_id, props).await.expect("upsert");
        assert!(
            storage.has_node(node_id).await.expect("has_node"),
            "parameterized has_node must find exact id"
        );

        let node = storage.get_node(node_id).await.expect("get_node").unwrap();
        assert_eq!(node.id, node_id);

        storage.delete_node(node_id).await.expect("delete");
        assert!(
            !storage
                .has_node(node_id)
                .await
                .expect("has_node after delete"),
            "delete must remove only the targeted node"
        );

        storage
            .upsert_node("SAFE_NODE", HashMap::new())
            .await
            .expect("safe upsert");
        assert!(storage.has_node("SAFE_NODE").await.unwrap());
    }
}

#[test]
fn spec022_nodes_ops_use_parameterized_cypher() {
    let nodes = include_str!("../src/adapters/postgres/graph/nodes_ops.rs");
    assert!(
        nodes.matches("cypher_query_bound").count() >= 2,
        "pg_has_node and pg_get_node must use parameterized Cypher"
    );
    assert!(
        nodes.contains("cypher_execute_bound"),
        "pg_delete_node must use parameterized Cypher execute"
    );
}

#[test]
fn spec022_edges_ops_use_parameterized_cypher() {
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(edges.contains("cypher_query_bound"));
    assert!(edges.contains("cypher_execute_bound"));
}

#[test]
fn spec022_cypher_exec_exposes_bound_helpers() {
    let src = include_str!("../src/adapters/postgres/graph/helpers/cypher_exec.rs");

    assert!(src.contains("cypher_query_bound"));
    assert!(src.contains("cypher_execute_bound"));
    assert!(src.contains("fn cypher_bound_sql"));
    assert!(src.contains("PgAgtype::from_json"));

    assert!(
        !src.contains("params_lit}'::agtype"),
        "SPEC-044: inline agtype literal third arg is forbidden (C-1)"
    );
    let bound_sql = src
        .split("fn cypher_bound_sql")
        .nth(1)
        .and_then(|tail| tail.split("async fn cypher_query_bound").next())
        .unwrap_or("");
    assert!(
        !bound_sql.contains("$1::agtype"),
        "SPEC-044: $1::agtype cast is forbidden in bound SQL builder (C-2)"
    );
    assert!(
        src.contains(", $1)"),
        "SPEC-044: bound Cypher must use bare $1 third argument"
    );

    let execute_bound = src
        .split("async fn cypher_execute_bound")
        .nth(1)
        .and_then(|tail| tail.split("// ── non-parameterized").next())
        .expect("cypher_execute_bound block");
    assert!(
        !execute_bound.contains("raw_sql"),
        "SPEC-044: cypher_execute_bound must use sqlx::query + bind (C-3)"
    );
}
