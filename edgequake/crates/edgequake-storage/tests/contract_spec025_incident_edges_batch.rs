//! SPEC-025 6.2 / SPEC-053 — batched incident edge lookup contract.

use std::collections::HashMap;

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

#[test]
fn contract_incident_edges_batch_uses_edge_child_table() {
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");

    // SPEC-053: query must target "EDGE" child table directly (has btree property indexes).
    assert!(
        edges.contains(r#""EDGE""#),
        "incident edges batch must query the \"EDGE\" child table (not parent)"
    );

    // SPEC-053: source and target extracted from edge properties (no vertex JOIN).
    assert!(
        edges.contains("source_id"),
        "incident edges batch must filter by source_id property"
    );
    assert!(
        edges.contains("target_id"),
        "incident edges batch must filter by target_id property"
    );

    // SPEC-053.1: must use OR (not UNION).
    // agtype_to_json() returns `json` (not `jsonb`). PostgreSQL `json` type has no
    // equality operator, so UNION (which deduplicates via equality) raises
    // "could not identify an equality operator for type json".
    // OR resolves via BitmapOr of two index scans without comparing json values.
    assert!(
        !edges.contains(" UNION \n") && !edges.contains(" UNION \\"),
        "incident edges batch must NOT use UNION on a json column (no equality operator)"
    );
    assert!(
        edges.contains("OR {tgt} IN")
            || edges.contains("OR ag_catalog")
            || edges.contains("OR e.source_id"),
        "incident edges batch must use OR (BitmapOr) to combine source_id / target_id scans"
    );

    // SPEC-053: must NOT query the parent vertex table (no indexes — M070 dropped them).
    assert!(
        !edges.contains("_ag_label_vertex"),
        "incident edges batch must not query the AGE parent vertex table"
    );

    // SPEC-025 original: must not use Cypher UNWIND fallback.
    assert!(
        !edges.contains("UNWIND [{}] AS nid MATCH"),
        "pg_get_incident_edges_batch must not use Cypher UNWIND"
    );
}

/// SPEC-053: node_degrees_batch must also use "EDGE" child table, not parent tables.
#[test]
fn contract_node_degrees_batch_uses_edge_child_table() {
    let nodes = include_str!("../src/adapters/postgres/graph/nodes_ops/read.rs");

    // Must query "EDGE" child table for degree counting.
    assert!(
        nodes.contains(r#""EDGE""#),
        "pg_node_degrees_batch must query the \"EDGE\" child table"
    );

    // The VALUES CTE replaces the _ag_label_vertex lookup for degree-0 nodes.
    assert!(
        nodes.contains("VALUES"),
        "pg_node_degrees_batch must use a VALUES CTE instead of _ag_label_vertex lookup"
    );
}

#[tokio::test]
async fn contract_spec025_incident_edges_batch_matches_per_node_union() {
    let graph = MemoryGraphStorage::new("batch-contract");
    graph.initialize().await.unwrap();
    for (src, tgt) in [("A", "B"), ("B", "C"), ("A", "D")] {
        graph.upsert_edge(src, tgt, HashMap::new()).await.unwrap();
    }

    let view = GraphReadView::new(&graph);
    let node_ids = vec!["A".to_string(), "B".to_string()];
    let batch = view
        .get_incident_edges_batch(&node_ids, None, None)
        .await
        .unwrap();

    let mut per_node = Vec::new();
    for id in &node_ids {
        per_node.extend(view.get_node_edges(id).await.unwrap());
    }

    let batch_keys: std::collections::HashSet<_> = batch
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();
    let per_node_keys: std::collections::HashSet<_> = per_node
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();

    assert_eq!(batch_keys, per_node_keys);
    assert_eq!(batch.len(), 3, "A-B, A-D, B-C");
}

/// SPEC-053: Verify degree-0 nodes are included in pg_node_degrees_batch results.
#[tokio::test]
async fn contract_spec053_degree_zero_nodes_returned() {
    let graph = MemoryGraphStorage::new("degree-zero-contract");
    graph.initialize().await.unwrap();

    // Insert only one edge: A → B; node C is isolated.
    graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();

    let view = GraphReadView::new(&graph);
    let degrees = view
        .node_degrees_batch(&["A".to_string(), "B".to_string(), "C".to_string()])
        .await
        .unwrap();

    let map: HashMap<_, _> = degrees.into_iter().collect();

    // A has out-degree 1; B has in-degree 1; C is isolated (0).
    assert_eq!(*map.get("A").unwrap_or(&0), 1, "A has degree 1");
    assert_eq!(*map.get("B").unwrap_or(&0), 1, "B has degree 1");
    assert_eq!(*map.get("C").unwrap_or(&0), 0, "C has degree 0 (isolated)");
}
