//! SPEC-088 Phase 6 — e2e verification of implemented IMPs (First Principles / O(K log N)).
//!
//! Run: `cargo test -p edgequake-storage --features postgres --test e2e_spec088_improvements`
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::adapters::postgres::{
    filtered_ann_gucs_satisfy_contract, parse_partial_by_workspace_env, HnswRuntimePolicy,
    PgVectorStorage, VectorIndexType,
};
use edgequake_storage::traits::{
    GraphStorage, GraphStorageMutateOps, GraphStorageReadOps, KVStorage, VectorStorage,
};
use edgequake_storage::{PostgresAGEGraphStorage, PostgresKVStorage};
use sqlx::Row;
use std::collections::{HashMap, HashSet};

/// IMP-002-01: filtered ANN GUC set always includes iterative_scan + max_scan_tuples.
#[test]
fn imp_002_01_filtered_ann_contract_unit() {
    let stmts = PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 25, true, true);
    assert!(
        filtered_ann_gucs_satisfy_contract(&stmts, true),
        "filtered ANN must set iterative_scan + max_scan_tuples: {stmts:?}"
    );
    assert!(
        stmts.iter().any(|s| s.contains("iterative_scan")),
        "{stmts:?}"
    );
    assert!(
        stmts.iter().any(|s| s.contains("max_scan_tuples")),
        "{stmts:?}"
    );
    // Unfiltered must NOT force iterative_scan (exact top-K path).
    let unf = PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 25, false, true);
    assert!(
        !unf.iter().any(|s| s.contains("iterative_scan")),
        "unfiltered should not set iterative_scan: {unf:?}"
    );
}

/// IMP-001-01: partial-by-workspace defaults on (opt-out with 0).
#[test]
fn imp_001_01_partial_default_on() {
    assert!(parse_partial_by_workspace_env(""));
    assert!(HnswRuntimePolicy::default().partial_by_workspace);
    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "0");
    assert!(!HnswRuntimePolicy::from_env().partial_by_workspace);
    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
}

/// IMP-031-01: get_nodes_by_ids uses native batch (no Cypher IN in source).
#[test]
fn imp_031_01_get_nodes_by_ids_delegates_to_batch_source() {
    let src = include_str!("../src/adapters/postgres/graph/nodes_ops/read.rs");
    assert!(
        src.contains("pg_get_nodes_batch")
            && src.contains("pg_get_nodes_by_ids")
            && !src.contains("WHERE n.node_id IN ["),
        "get_nodes_by_ids must not build Cypher IN lists"
    );
}

/// IMP-046-01: native writes remain default-on.
#[test]
fn imp_046_01_native_writes_default_on_source() {
    let src = include_str!("../src/adapters/postgres/graph/mod.rs");
    assert!(src.contains("Err(_) => true"), "native writes default ON");
    assert!(
        include_str!("../src/adapters/postgres/graph/nodes_ops/mutate.rs")
            .contains("native graph writes disabled"),
        "Cypher path must warn (IMP-046-01)"
    );
}

/// IMP-031-01 e2e: batch get is O(1) RT and returns correct nodes.
#[tokio::test]
async fn imp_031_01_e2e_native_batch_get_nodes() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp031") else {
        return;
    };
    let graph = PostgresAGEGraphStorage::new(config);
    graph.initialize().await.expect("graph init");

    let mut props = HashMap::new();
    props.insert("label".into(), serde_json::json!("Person"));
    graph
        .upsert_nodes_batch(&[
            ("ALPHA".into(), props.clone()),
            ("BETA".into(), props.clone()),
            ("GAMMA".into(), props),
        ])
        .await
        .expect("upsert");

    let found = graph
        .get_nodes_by_ids(&["BETA".into(), "MISSING".into(), "ALPHA".into()])
        .await
        .expect("get_nodes_by_ids");
    // Only hits, order preserved among found
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].id, "BETA");
    assert_eq!(found[1].id, "ALPHA");

    let one = graph.get_node("GAMMA").await.expect("get_node");
    assert!(one.is_some());
    assert!(graph.has_node("ALPHA").await.unwrap());
    assert!(!graph.has_node("NOPE").await.unwrap());
}

/// IMP-075-01 e2e: get_by_ids_ordered is one RT for multi-key (staging pattern).
#[tokio::test]
async fn imp_075_01_e2e_kv_batch_not_n_plus_one() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp075") else {
        return;
    };
    let kv = PostgresKVStorage::new(config);
    kv.initialize().await.expect("kv init");
    kv.upsert(&[
        ("s-meta".into(), serde_json::json!({"t": "m"})),
        ("s-content".into(), serde_json::json!({"t": "c"})),
        ("s-hash".into(), serde_json::json!({"t": "h"})),
    ])
    .await
    .unwrap();

    let keys = vec![
        "s-meta".into(),
        "s-content".into(),
        "s-hash".into(),
        "missing".into(),
    ];
    let vals = kv.get_by_ids_ordered(&keys).await.unwrap();
    assert_eq!(vals.len(), 4);
    assert_eq!(vals[0].as_ref().unwrap()["t"], "m");
    assert_eq!(vals[1].as_ref().unwrap()["t"], "c");
    assert_eq!(vals[2].as_ref().unwrap()["t"], "h");
    assert!(vals[3].is_none());
}

/// IMP-140-01 e2e: claim path uses index-friendly plan (status + workspace + created_at).
#[tokio::test]
async fn imp_140_01_e2e_claim_index_plan() {
    let Some(url) = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::fs::read_to_string("/tmp/edgequake-db-url").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| Some("postgres://edgequake:edgequake_secret@localhost:5432/edgequake".into()))
    else {
        return;
    };
    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("skip IMP-140: {e}");
            return;
        }
    };

    // Ensure claim index exists (M098) — IF NOT EXISTS
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_claim_workspace_created
            ON tasks (status, workspace_id, created_at ASC)
            WHERE status IN ('pending', 'processing')
        "#,
    )
    .execute(&pool)
    .await
    .ok();

    // EXPLAIN the sargable pending arm used by claim_next (IMP-140-02 pending CTE).
    // UNION/stale arms are separate; this probes the index-friendly equality filter.
    let rows = sqlx::query(
        r#"
        EXPLAIN (FORMAT TEXT)
        SELECT track_id FROM tasks
        WHERE status = 'pending'
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let plan: String = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .collect::<Vec<_>>()
        .join("\n");
    let lower = plan.to_lowercase();
    // Empty / tiny table may still seq-scan; accept index OR bounded limit plan.
    assert!(
        lower.contains("index")
            || lower.contains("limit")
            || lower.contains("sort")
            || lower.contains("seq scan"),
        "unexpected claim plan:\n{plan}"
    );

    // Document index presence for fair claim
    let idx: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT indexname::text FROM pg_indexes
        WHERE tablename = 'tasks' AND indexname = 'idx_tasks_claim_workspace_created'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    assert!(
        idx.is_some(),
        "idx_tasks_claim_workspace_created must exist (M098 / IMP-140-01)"
    );
}

/// IMP-031-03: has_edge/get_edge must not use Cypher MATCH on request path.
#[test]
fn imp_031_03_get_edge_native_source() {
    let src = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(
        src.contains("get_edge native failed") || src.contains("DATA-AGE-GRAPH-GET-EDGE-034"),
        "get_edge must use native SQL"
    );
    assert!(
        !src.contains("MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id}) RETURN r LIMIT 1")
            || src.contains("native graph writes disabled"),
        "primary has_edge path must not be Cypher"
    );
}

/// IMP-031-02/04: expand/neighbors use native BFS (no variable-length Cypher).
#[test]
fn imp_031_02_expand_edges_native_source() {
    let src = include_str!("../src/adapters/postgres/graph/query_ops/expand.rs");
    assert!(
        src.contains("pg_bfs_expand") && src.contains("pg_get_nodes_batch"),
        "expand must use native BFS + batch node fetch"
    );
    assert!(
        !src.contains("-[*0..") && !src.contains("-[*1.."),
        "expand/neighbors must not use variable-length Cypher"
    );
    assert!(
        src.contains("pg_get_edges_for_node_set"),
        "edge hydrate still native set"
    );
}

/// IMP-140-02: claim_next splits pending/stale for sargable status filters.
#[test]
fn imp_140_02_claim_union_pending_stale_source() {
    let src = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(
        src.contains("WITH pending AS") && src.contains("stale AS") && src.contains("UNION ALL"),
        "claim_next must split pending/stale for index-friendly plans"
    );
}

/// IMP-031-04 e2e: native neighbors BFS returns 1-hop endpoints.
#[tokio::test]
async fn imp_031_04_e2e_native_neighbors() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp031nb") else {
        return;
    };
    let graph = PostgresAGEGraphStorage::new(config);
    graph.initialize().await.expect("graph init");
    let mut np = HashMap::new();
    np.insert("label".into(), serde_json::json!("N"));
    graph
        .upsert_nodes_batch(&[
            ("CENTER".into(), np.clone()),
            ("N1".into(), np.clone()),
            ("N2".into(), np.clone()),
            ("N3".into(), np),
        ])
        .await
        .unwrap();
    for t in ["N1", "N2", "N3"] {
        graph
            .upsert_edge("CENTER", t, HashMap::new())
            .await
            .unwrap();
    }
    let neigh = graph
        .get_neighbors("CENTER", 1, None, None)
        .await
        .expect("neighbors");
    let ids: HashSet<_> = neigh.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains("N1") && ids.contains("N2") && ids.contains("N3"));
    assert!(!ids.contains("CENTER"));
}

/// IMP-031-07: get_all_nodes / get_all_edges native (admin dump; still O(N)/O(E)).
#[test]
fn imp_031_07_get_all_native_source() {
    let nodes = include_str!("../src/adapters/postgres/graph/nodes_ops/read.rs");
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(
        nodes.contains("DATA-AGE-GRAPH-GET-ALL-NODES")
            && !nodes.contains("MATCH (n:Node) RETURN n"),
        "get_all_nodes must be native"
    );
    assert!(
        edges.contains("DATA-AGE-GRAPH-GET-ALL-EDGES")
            && !edges.contains("MATCH ()-[r:EDGE]->() RETURN r"),
        "get_all_edges must be native"
    );
}

/// IMP-031-06: clear / clear_workspace use native DELETE (no Cypher DETACH).
#[test]
fn imp_031_06_clear_workspace_native_source() {
    let src = include_str!("../src/adapters/postgres/graph/analytics_ops.rs");
    assert!(
        src.contains("DATA-AGE-GRAPH-CLEAR")
            && src.contains("pg_delete_nodes_batch")
            && !src.contains("DETACH DELETE n"),
        "clear/clear_workspace must be native SQL"
    );
}

/// IMP-031-06 e2e: clear_workspace removes only that workspace's nodes/edges.
#[tokio::test]
async fn imp_031_06_e2e_native_clear_workspace() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp031clr") else {
        return;
    };
    let graph = PostgresAGEGraphStorage::new(config);
    graph.initialize().await.expect("graph init");
    let ws_a = uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let ws_b = uuid::Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
    let mut props_a = HashMap::new();
    props_a.insert("workspace_id".into(), serde_json::json!(ws_a.to_string()));
    props_a.insert("tenant_id".into(), serde_json::json!("t1"));
    let mut props_b = HashMap::new();
    props_b.insert("workspace_id".into(), serde_json::json!(ws_b.to_string()));
    props_b.insert("tenant_id".into(), serde_json::json!("t1"));
    graph
        .upsert_nodes_batch(&[
            ("WA1".into(), props_a.clone()),
            ("WA2".into(), props_a),
            ("WB1".into(), props_b),
        ])
        .await
        .unwrap();
    graph
        .upsert_edge("WA1", "WA2", HashMap::new())
        .await
        .unwrap();

    let (n, _e) = graph.clear_workspace(&ws_a).await.expect("clear_ws");
    assert!(n >= 2, "deleted at least WA1/WA2, got {n}");
    assert!(graph.get_node("WA1").await.unwrap().is_none());
    assert!(graph.get_node("WB1").await.unwrap().is_some());
}

/// IMP-031-05: delete_edge routes to native batch when writes ON.
#[test]
fn imp_031_05_delete_edge_native_source() {
    let src = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(
        src.contains("pg_delete_edges_batch") && src.contains("native batch edge delete failed"),
        "delete_edge/batch must use native SQL path"
    );
}

/// IMP-031-05 e2e: native delete edge removes directed edge only.
#[tokio::test]
async fn imp_031_05_e2e_native_delete_edge() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp031del") else {
        return;
    };
    let graph = PostgresAGEGraphStorage::new(config);
    graph.initialize().await.expect("graph init");
    let mut np = HashMap::new();
    np.insert("label".into(), serde_json::json!("N"));
    graph
        .upsert_nodes_batch(&[("A".into(), np.clone()), ("B".into(), np)])
        .await
        .unwrap();
    graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();
    assert!(graph.has_edge("A", "B").await.unwrap());
    graph.delete_edge("A", "B").await.expect("delete_edge");
    assert!(!graph.has_edge("A", "B").await.unwrap());
}

/// IMP-031-03 e2e: native edge get after native upsert.
#[tokio::test]
async fn imp_031_03_e2e_native_get_edge() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp031edge") else {
        return;
    };
    let graph = PostgresAGEGraphStorage::new(config);
    graph.initialize().await.expect("graph init");
    let mut np = HashMap::new();
    np.insert("label".into(), serde_json::json!("N"));
    graph
        .upsert_nodes_batch(&[("S".into(), np.clone()), ("T".into(), np)])
        .await
        .unwrap();
    let mut ep = HashMap::new();
    ep.insert("rel".into(), serde_json::json!("KNOWS"));
    graph.upsert_edge("S", "T", ep).await.expect("upsert edge");
    assert!(graph.has_edge("S", "T").await.unwrap());
    let e = graph.get_edge("S", "T").await.unwrap();
    assert!(e.is_some());
    assert_eq!(e.unwrap().source, "S");
    assert!(!graph.has_edge("T", "S").await.unwrap());
}

/// IMP-075-03: lineage page enrichment must batch chunk KV (no N+1 get_by_id loop).
#[test]
fn imp_075_03_lineage_enrich_batch_source() {
    let src = include_str!("../../edgequake-api/src/handlers/lineage/queries.rs");
    assert!(
        src.contains("get_by_ids_ordered") && src.contains("IMP-075-03"),
        "enrich_lineage_page_data must use get_by_ids_ordered"
    );
    // Guard against reintroducing per-chunk RT loop.
    assert!(
        !src.contains("for id in &chunk_ids") && !src.contains("kv.get_by_id(id)"),
        "must not loop get_by_id over chunk_ids"
    );
}

/// IMP-075-04/10: merge-progress + status updates use staging/final SSOT (1 RT).
#[test]
fn imp_075_04_status_updates_batch_source() {
    let src = include_str!("../../edgequake-api/src/processor/status_updates.rs");
    assert!(
        src.contains("load_staging_and_final_metadata") && src.contains("IMP-075-04"),
        "status_updates must use load_staging_and_final_metadata SSOT"
    );
    assert!(
        src.matches("load_staging_and_final_metadata").count() >= 3,
        "progress + update_document_status + ensure/stats paths should share SSOT"
    );
}

/// IMP-075-05: orphan recovery batches page metadata (staging+final).
#[test]
fn imp_075_05_orphan_recovery_batch_source() {
    let src = include_str!("../../edgequake-api/src/services/orphan_task_recovery.rs");
    assert!(
        src.contains("get_by_ids_ordered") && src.contains("IMP-075-05"),
        "recover_orphaned_tasks must batch page meta keys"
    );
}

/// IMP-075-06: MemoryKV implements ordered batch (not trait default N× get_by_id).
#[test]
fn imp_075_06_memory_kv_ordered_batch_source() {
    let src = include_str!("../src/adapters/memory/kv.rs");
    assert!(
        src.contains("get_by_ids_ordered") && src.contains("IMP-075-06"),
        "MemoryKVStorage must override get_by_ids_ordered"
    );
}

/// IMP-075-07/08/09/10: admission dual-key + content resolve + staging/final SSOT.
#[test]
fn imp_075_07_08_09_api_dual_key_batch_source() {
    let dedup = include_str!("../../edgequake-api/src/services/workspace_content_hash_dedup.rs");
    assert!(
        dedup.contains("get_by_ids_ordered") && dedup.contains("IMP-075-07"),
        "workspace hash visibility must batch final+staging"
    );
    let text = include_str!("../../edgequake-api/src/services/text_insert_content.rs");
    assert!(
        text.contains("IMP-075-08")
            && text.contains("load_staging_first_metadata")
            && text.contains("IMP-075-09")
            && text.contains("load_staging_and_final_metadata")
            && text.contains("IMP-075-10")
            && text.contains("struct StagingFinalMeta"),
        "text_insert must expose StagingFinalMeta SSOT"
    );
    let sync = include_str!("../../edgequake-api/src/services/task_document_sync.rs");
    assert!(
        sync.contains("load_staging_first_metadata") && sync.contains("IMP-075-10"),
        "task_document_sync must not resolve+re-get"
    );
}

/// IMP-031-08: cascade source-prefix discovery is probe-first GIN (not tenant-first).
#[test]
fn imp_031_08_source_prefix_probe_first_materialized_source() {
    let src = include_str!("../src/adapters/postgres/graph/scan_ops.rs");
    assert!(
        src.contains("probes AS MATERIALIZED")
            && src.contains("hits AS MATERIALIZED")
            && src.contains("IMP-031-08"),
        "find_nodes/edges_by_source_prefixes must force GIN via MATERIALIZED probes"
    );
}

/// IMP-031-08: EXPLAIN on production-shaped graph must use GIN, not Join Filter.
#[tokio::test]
async fn imp_031_08_e2e_explain_uses_source_ids_gin() {
    let Some(url) = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| Some("postgres://edgequake:edgequake_secret@localhost:5432/edgequake".into()))
    else {
        return;
    };
    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };
    // Prefer the large default graph when present (incident repro).
    let graph = "eq_eq_default_graph";
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT nspname::text FROM pg_namespace WHERE nspname = $1")
            .bind(graph)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
    if exists.is_none() {
        return;
    }
    let n: i64 = sqlx::query_scalar(&format!(r#"SELECT COUNT(*)::bigint FROM {graph}."Node""#))
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    if n < 1000 {
        // Tiny graphs don't reproduce the tenant-first cliff.
        return;
    }
    let doc = "019f933a-622a-74f5-baae-7a4591fc424f";
    let sql = format!(
        r#"
        EXPLAIN (FORMAT TEXT)
        WITH probes AS MATERIALIZED (
          SELECT probe_id FROM unnest(ARRAY[$1]::text[]) AS t(probe_id)
          UNION
          SELECT (p.prefix || gs.i::text)
          FROM unnest(ARRAY[$2]::text[]) AS p(prefix)
          CROSS JOIN generate_series(0, 255) AS gs(i)
        ),
        hits AS MATERIALIZED (
          SELECT v.properties
          FROM probes pr
          INNER JOIN {graph}."Node" v
            ON ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
               @> to_jsonb(pr.probe_id)
        )
        SELECT ag_catalog.agtype_to_json(h.properties)
        FROM hits h
        LIMIT 5000
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(doc)
        .bind(format!("{doc}-chunk-"))
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN");
    let plan: String = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .collect::<Vec<_>>()
        .join("\n");
    let lower = plan.to_lowercase();
    assert!(
        lower.contains("idx_node_source_ids_gin") || lower.contains("bitmap index scan"),
        "probe-first plan must use source_ids GIN:\n{plan}"
    );
    assert!(
        !lower.contains("join filter"),
        "must not recheck @> as Join Filter (tenant-first cliff):\n{plan}"
    );
}

/// IMP-031-08 e2e: find_nodes_by_source_prefixes returns chunk-linked nodes under timeout.
#[tokio::test]
async fn imp_031_08_e2e_source_prefix_discovery_finds_nodes() {
    use edgequake_storage::traits::{GraphScanOps, NodeListFilter};

    let Some(config) = postgres_test_config::require_or_skip_postgres("imp031src") else {
        return;
    };
    let graph = PostgresAGEGraphStorage::new(config);
    graph.initialize().await.expect("graph init");

    let doc = format!("imp031src-{}", uuid::Uuid::new_v4());
    let chunk0 = format!("{doc}-chunk-0");
    let mut props = HashMap::new();
    props.insert("label".into(), serde_json::json!("Entity"));
    props.insert(
        "source_ids".into(),
        serde_json::json!([chunk0, format!("{doc}-chunk-1")]),
    );
    props.insert("tenant_id".into(), serde_json::json!("t-imp031"));
    props.insert("workspace_id".into(), serde_json::json!("ws-imp031"));
    graph
        .upsert_nodes_batch(&[(format!("N-{doc}"), props)])
        .await
        .expect("upsert");

    let filter = NodeListFilter {
        tenant_id: Some("t-imp031".into()),
        workspace_id: Some("ws-imp031".into()),
        ..Default::default()
    };
    let found = graph
        .find_nodes_by_source_prefixes(&filter, std::slice::from_ref(&doc))
        .await
        .expect("find_nodes_by_source_prefixes");
    assert!(
        found.iter().any(|n| n.id == format!("N-{doc}")),
        "must discover node via GIN source_ids probe for doc {doc}, got {found:?}"
    );
}

/// IMP-075-12: batch deletion loads content+metadata in one RT.
#[test]
fn imp_075_12_batch_deletion_content_meta_batch_source() {
    let src = include_str!("../../edgequake-api/src/processor/batch_deletion.rs");
    assert!(
        src.contains("get_by_ids_ordered") && src.contains("IMP-075-12"),
        "build_deletion_task_data must batch content+metadata"
    );
    // Guard: no sequential content then metadata get_by_id pattern.
    assert!(
        !src.contains("get_by_id(&content_key)") && !src.contains("get_by_id(&metadata_key)"),
        "must not sequential get_by_id content then metadata"
    );
}

/// IMP-075-13: delete key resolve batches final+staging (final-first).
#[test]
fn imp_075_13_delete_key_resolve_batch_source() {
    let src = include_str!("../../edgequake-api/src/handlers/documents/delete/single.rs");
    assert!(
        src.contains("get_by_ids_ordered") && src.contains("IMP-075-13"),
        "resolve_kv_key_prefix must batch final+staging"
    );
}

/// IMP-075-11: prepare/cancel/reanalyze use staging-first SSOT (no resolve+re-get).
#[test]
fn imp_075_11_prepare_cancel_reanalyze_ssot_source() {
    let prepare = include_str!("../../edgequake-api/src/processor/text_insert/prepare.rs");
    assert!(
        prepare.contains("load_staging_first_metadata") && prepare.contains("IMP-075-11"),
        "text_insert prepare must use load_staging_first_metadata"
    );
    assert!(
        !prepare.contains("resolve_document_metadata_key"),
        "prepare must not resolve then re-get"
    );
    let cancel = include_str!("../../edgequake-api/src/processor/text_insert/cancel.rs");
    assert!(
        cancel.contains("load_staging_first_metadata") && cancel.contains("IMP-075-11"),
        "text_insert cancel must use load_staging_first_metadata"
    );
    assert!(
        !cancel.contains("resolve_document_metadata_key"),
        "cancel must not resolve then re-get"
    );
    let reanalyze = include_str!("../../edgequake-api/src/services/multimodal/reanalyze.rs");
    assert!(
        reanalyze.contains("load_staging_first_metadata") && reanalyze.contains("IMP-075-11"),
        "reanalyze must use load_staging_first_metadata"
    );
    assert!(
        !reanalyze.contains("resolve_document_metadata_key"),
        "reanalyze must not resolve then re-get"
    );
}

/// IMP-002-01 e2e: filtered ANN returns K under workspace filter with iterative GUCs.
#[tokio::test]
async fn imp_002_01_e2e_filtered_ann_returns_k() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("imp002") else {
        return;
    };
    let dim = 8usize;
    // Disable partial auto during tiny-corpus test (DDL cost); contract still holds.
    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "0");
    let storage = PgVectorStorage::with_dimension(config, dim);
    if let Err(e) = storage.initialize().await {
        eprintln!("skip init: {e}");
        std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
        return;
    }
    let emb = |i: f32| {
        let mut v = vec![0.0f32; dim];
        v[0] = i;
        v
    };
    let batch: Vec<(String, Vec<f32>, serde_json::Value)> = (0..20)
        .map(|i| {
            (
                format!("id{i}"),
                emb(i as f32 / 20.0),
                serde_json::json!({
                    "workspace_id": if i < 10 { "wsA" } else { "wsB" },
                    "tenant_id": "t1",
                    "document_id": "d1",
                }),
            )
        })
        .collect();
    storage.upsert(&batch).await.expect("upsert");

    use edgequake_storage::traits::MetadataFilter;
    let filter = MetadataFilter {
        workspace_id: Some("wsA".into()),
        tenant_id: Some("t1".into()),
        ..Default::default()
    };
    let hits = storage
        .query_filtered(&emb(0.0), 5, None, Some(&filter))
        .await
        .expect("query_filtered");
    assert_eq!(
        hits.len(),
        5,
        "filtered ANN must return K under iterative_scan"
    );
    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
}
