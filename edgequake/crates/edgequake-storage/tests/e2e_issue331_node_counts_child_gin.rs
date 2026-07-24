//! SPEC-084 / GH-331 — node_counts_by_source_prefixes must hit child "Node" GIN.
//!
//! Run:
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! cargo test -p edgequake-storage --features postgres --test e2e_issue331_node_counts_child_gin -- --nocapture
//! ```

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{
    GraphScanOps, GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps, NodeListFilter,
};
use edgequake_storage::PostgresAGEGraphStorage;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

fn node_props(id: &str, prefix: &str) -> (String, HashMap<String, serde_json::Value>) {
    let mut props = HashMap::new();
    props.insert("node_id".into(), json!(id));
    props.insert("entity_type".into(), json!("CONCEPT"));
    props.insert(
        "source_ids".into(),
        json!([format!("{prefix}0"), format!("{prefix}1")]),
    );
    props.insert("tenant_id".into(), json!("t-issue331"));
    props.insert("workspace_id".into(), json!("ws-issue331"));
    (id.to_string(), props)
}

#[tokio::test]
async fn issue331_node_counts_uses_child_gin_explain() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("issue331_explain") else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = PostgresAGEGraphStorage::new(config.clone());
    graph.initialize().await.expect("init");
    let graph_name = graph.graph_name().to_string();

    let prefix = "issue331-doc-chunk-";
    graph
        .upsert_nodes_batch(&[node_props("ISSUE331_N0", prefix)])
        .await
        .expect("upsert");

    let pool = postgres_test_config::contract_pg_pool(&config).await;
    // Mirror production count join shape (analytics_ops.rs).
    let sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           WITH probes AS (
             SELECT (p.prefix || gs.i::text) AS chunk_id
             FROM unnest($1::text[]) AS p(prefix)
             CROSS JOIN generate_series(0, 3) AS gs(i)
           )
           SELECT count(DISTINCT v.id)::BIGINT
           FROM probes pr
           JOIN {graph}."Node" v
             ON ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
                @> to_jsonb(pr.chunk_id)"#,
        graph = graph_name
    );
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(vec![prefix.to_string()])
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.to_lowercase().contains("idx_node_source_ids_gin")
            || plan.to_lowercase().contains("bitmap")
            || plan.contains("Index Scan")
            || plan.to_lowercase().contains("gin"),
        "EXPLAIN must use Node source_ids GIN; plan:\n{plan}"
    );
    assert!(
        !plan.contains("_ag_label_vertex"),
        "must not plan against parent _ag_label_vertex; plan:\n{plan}"
    );

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
}

#[tokio::test]
async fn issue331_parity_count_vs_discovery() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("issue331_parity") else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = Arc::new(PostgresAGEGraphStorage::new(config.clone()));
    graph.initialize().await.expect("init");

    let mut nodes = Vec::new();
    let mut prefixes = Vec::new();
    for i in 0..8 {
        let prefix = format!("issue331-parity-{i}-chunk-");
        prefixes.push(prefix.clone());
        nodes.push(node_props(&format!("ISSUE331_P{i}"), &prefix));
    }
    graph.upsert_nodes_batch(&nodes).await.expect("upsert");

    let filter = NodeListFilter {
        tenant_id: Some("t-issue331".into()),
        workspace_id: Some("ws-issue331".into()),
        entity_type: None,
        search: None,
        community_ids: None,
    };
    let counts = graph
        .node_counts_by_source_prefixes(&prefixes)
        .await
        .expect("counts");
    for prefix in &prefixes {
        let found = graph
            .find_nodes_by_source_prefixes(&filter, std::slice::from_ref(prefix))
            .await
            .expect("discover");
        let cnt = *counts.get(prefix).unwrap_or(&0);
        assert_eq!(
            cnt as usize,
            found.len(),
            "count/discovery parity for {prefix}: count={cnt} discovery={}",
            found.len()
        );
    }

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
}

#[tokio::test]
async fn issue331_concurrent_reprocess_pool_stable() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("issue331_pool") else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = Arc::new(PostgresAGEGraphStorage::new(config.clone()));
    graph.initialize().await.expect("init");

    let mut nodes = Vec::new();
    let mut prefixes = Vec::new();
    for i in 0..40 {
        let prefix = format!("issue331-pool-{i}-chunk-");
        prefixes.push(prefix.clone());
        nodes.push(node_props(&format!("ISSUE331_POOL_{i}"), &prefix));
    }
    graph.upsert_nodes_batch(&nodes).await.expect("upsert");

    let start = Instant::now();
    let mut handles = Vec::new();
    for chunk in prefixes.chunks(5) {
        let g = Arc::clone(&graph);
        let p = chunk.to_vec();
        handles.push(tokio::spawn(async move {
            g.node_counts_by_source_prefixes(&p).await
        }));
    }
    for h in handles {
        h.await
            .expect("join")
            .expect("concurrent count must not pool-timeout");
    }
    assert!(
        start.elapsed().as_secs() < 30,
        "concurrent counts should finish quickly with child GIN"
    );

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
}
