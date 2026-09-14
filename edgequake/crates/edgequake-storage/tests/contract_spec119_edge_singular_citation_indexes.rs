//! SPEC-119 — singular edge citation btrees present + EXPLAIN Index Cond (LAW-119-2/7).
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps};
use edgequake_storage::PostgresAGEGraphStorage;
use perf_harness::{assert_plan_uses_index, PlanKind};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::test]
async fn contract_spec119_edge_singular_citation_indexes_exist() {
    let Some(cfg) = require_or_skip_postgres("spec119_singular_indexes") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let graph = PostgresAGEGraphStorage::new(cfg);
    graph.initialize().await.expect("graph init");

    // Upsert triggers ensure_indexes (single-flight).
    let mut props = HashMap::new();
    props.insert("entity_type".into(), json!("concept"));
    props.insert(
        "source_chunk_id".into(),
        json!(format!("{}-chunk-0", Uuid::new_v4())),
    );
    graph
        .upsert_node(&format!("SPEC119_IDX_{}", Uuid::new_v4().simple()), props)
        .await
        .expect("upsert triggers ensure_indexes");

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT indexname::text FROM pg_indexes \
         WHERE indexname IN ('idx_edge_source_chunk_id', 'idx_edge_source_document_id')",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    assert!(
        names.iter().any(|n| n == "idx_edge_source_chunk_id"),
        "LAW-119-3: expected idx_edge_source_chunk_id after ensure_indexes; got {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "idx_edge_source_document_id"),
        "LAW-119-3: expected idx_edge_source_document_id after ensure_indexes; got {names:?}"
    );
}

#[tokio::test]
async fn contract_spec119_singular_probe_uses_index() {
    let Some(cfg) = require_or_skip_postgres("spec119_singular_explain") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let storage = PostgresAGEGraphStorage::new(cfg.clone());
    storage.initialize().await.expect("graph init");

    let doc = format!("spec119-{}", Uuid::new_v4().simple());
    let chunk = format!("{doc}-chunk-0");
    let a = format!("SPEC119_A_{}", Uuid::new_v4().simple());
    let b = format!("SPEC119_B_{}", Uuid::new_v4().simple());

    let mut node_props = HashMap::new();
    node_props.insert("entity_type".into(), json!("concept"));
    node_props.insert("tenant_id".into(), json!("t-spec119"));
    node_props.insert("workspace_id".into(), json!("ws-spec119"));
    storage
        .upsert_node(&a, node_props.clone())
        .await
        .expect("node a");
    storage.upsert_node(&b, node_props).await.expect("node b");

    let mut edge_props = HashMap::new();
    edge_props.insert("tenant_id".into(), json!("t-spec119"));
    edge_props.insert("workspace_id".into(), json!("ws-spec119"));
    edge_props.insert("relation_type".into(), json!("RELATED"));
    // Singular-only citation (Symptom F shape) — no source_ids array.
    edge_props.insert("source_chunk_id".into(), json!(&chunk));
    edge_props.insert("source_document_id".into(), json!(&doc));
    storage
        .upsert_edge(&a, &b, edge_props)
        .await
        .expect("singular edge");

    let graph = storage.graph_name().to_string();
    let sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT 1
           FROM {graph}."EDGE" e
           WHERE ag_catalog.agtype_to_json(e.properties)->>'source_chunk_id' = $1
           LIMIT 100"#
    );
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(&chunk)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN singular chunk_id");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !plan.contains("_ag_label_edge"),
        "EXPLAIN must target child EDGE, not AGE parent: {plan}"
    );
    assert_plan_uses_index(&plan, &[PlanKind::Btree, PlanKind::Bitmap, PlanKind::Index]);
    assert!(
        plan.contains("idx_edge_source_chunk_id")
            || plan.to_lowercase().contains("index cond")
            || plan.to_lowercase().contains("index scan"),
        "expected Index Cond on singular source_chunk_id; plan:\n{plan}"
    );
    eprintln!("OK SPEC-119 EXPLAIN singular source_chunk_id:\n{plan}");

    // EC-03 / EC-12: source_document_id btree
    let doc_sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT 1
           FROM {graph}."EDGE" e
           WHERE ag_catalog.agtype_to_json(e.properties)->>'source_document_id' = $1
           LIMIT 100"#
    );
    let doc_rows: Vec<(String,)> = sqlx::query_as(&doc_sql)
        .bind(&doc)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN singular document_id");
    let doc_plan = doc_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !doc_plan.contains("_ag_label_edge"),
        "document_id EXPLAIN must target child EDGE: {doc_plan}"
    );
    assert_plan_uses_index(
        &doc_plan,
        &[PlanKind::Btree, PlanKind::Bitmap, PlanKind::Index],
    );
    assert!(
        doc_plan.contains("idx_edge_source_document_id")
            || doc_plan.to_lowercase().contains("index cond"),
        "expected Index Cond on source_document_id; plan:\n{doc_plan}"
    );
    eprintln!("OK SPEC-119 EXPLAIN singular source_document_id:\n{doc_plan}");

    // EC-03 / production shape: `= ANY($1::text[])` (not OR+CTE IN — Seq Scan trap)
    let any_chunk_sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT 1
           FROM {graph}."EDGE" e
           WHERE ag_catalog.agtype_to_json(e.properties)->>'source_chunk_id' = ANY($1::text[])
           LIMIT 100"#
    );
    let any_chunk_rows: Vec<(String,)> = sqlx::query_as(&any_chunk_sql)
        .bind(vec![chunk.clone()])
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN = ANY source_chunk_id");
    let any_chunk_plan = any_chunk_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !any_chunk_plan.contains("_ag_label_edge"),
        "= ANY EXPLAIN must target child EDGE: {any_chunk_plan}"
    );
    assert_plan_uses_index(
        &any_chunk_plan,
        &[PlanKind::Btree, PlanKind::Bitmap, PlanKind::Index],
    );
    let any_chunk_lower = any_chunk_plan.to_lowercase();
    assert!(
        !any_chunk_lower.contains("seq scan"),
        "production singular = ANY(source_chunk_id) must not Seq Scan:\n{any_chunk_plan}"
    );
    assert!(
        any_chunk_plan.contains("idx_edge_source_chunk_id")
            || any_chunk_lower.contains("index cond"),
        "expected Index Cond on = ANY source_chunk_id; plan:\n{any_chunk_plan}"
    );
    eprintln!("OK SPEC-119 EXPLAIN = ANY source_chunk_id:\n{any_chunk_plan}");

    let any_doc_sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT 1
           FROM {graph}."EDGE" e
           WHERE ag_catalog.agtype_to_json(e.properties)->>'source_document_id' = ANY($1::text[])
           LIMIT 100"#
    );
    let any_doc_rows: Vec<(String,)> = sqlx::query_as(&any_doc_sql)
        .bind(vec![doc.clone()])
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN = ANY source_document_id");
    let any_doc_plan = any_doc_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !any_doc_plan.contains("_ag_label_edge"),
        "= ANY document_id EXPLAIN must target child EDGE: {any_doc_plan}"
    );
    assert_plan_uses_index(
        &any_doc_plan,
        &[PlanKind::Btree, PlanKind::Bitmap, PlanKind::Index],
    );
    let any_doc_lower = any_doc_plan.to_lowercase();
    assert!(
        !any_doc_lower.contains("seq scan"),
        "production singular = ANY(source_document_id) must not Seq Scan:\n{any_doc_plan}"
    );
    assert!(
        any_doc_plan.contains("idx_edge_source_document_id")
            || any_doc_lower.contains("index cond"),
        "expected Index Cond on = ANY source_document_id; plan:\n{any_doc_plan}"
    );
    eprintln!("OK SPEC-119 EXPLAIN = ANY source_document_id:\n{any_doc_plan}");

    // EC-03: OR of both singular props (legacy trap shape — must still prefer index)
    let or_sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT 1
           FROM {graph}."EDGE" e
           WHERE ag_catalog.agtype_to_json(e.properties)->>'source_chunk_id' = $1
              OR ag_catalog.agtype_to_json(e.properties)->>'source_document_id' = $2
           LIMIT 100"#
    );
    let or_rows: Vec<(String,)> = sqlx::query_as(&or_sql)
        .bind(&chunk)
        .bind(&doc)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN OR singular");
    let or_plan = or_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !or_plan.contains("_ag_label_edge"),
        "OR EXPLAIN must target child EDGE: {or_plan}"
    );
    assert_plan_uses_index(
        &or_plan,
        &[PlanKind::Btree, PlanKind::Bitmap, PlanKind::Index],
    );
    let or_lower = or_plan.to_lowercase();
    assert!(
        !or_lower.contains("seq scan") || or_lower.contains("index"),
        "OR singular probe must not be plain Seq Scan:\n{or_plan}"
    );
    eprintln!("OK SPEC-119 EXPLAIN OR singular citation props:\n{or_plan}");

    // LAW-119-2: ::jsonb cast must NOT be the serving filter shape.
    let cast_sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT 1
           FROM {graph}."EDGE" e
           WHERE (ag_catalog.agtype_to_json(e.properties))::jsonb->>'source_chunk_id' = $1
           LIMIT 100"#
    );
    let cast_rows: Vec<(String,)> = sqlx::query_as(&cast_sql)
        .bind(&chunk)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN cast shape");
    let cast_plan = cast_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    // Document the trap: casted form should not claim our btree (often Seq Scan).
    eprintln!("SPEC-119 cast-trap plan (must not be the code path):\n{cast_plan}");
}

#[test]
fn contract_spec119_singular_sql_source_has_no_jsonb_cast_on_arrow() {
    let src = include_str!("../src/adapters/postgres/graph/scan_ops.rs");
    // Narrow window: Symptom F singular block comments + filters.
    let start = src
        .find("SPEC-098 Symptom F: poisoned source_ids")
        .expect("singular block marker");
    let window = &src[start..];
    let end = window
        .find("Source-prefix singular edge query failed")
        .expect("singular error marker");
    let singular = &window[..end];
    assert!(
        !singular.contains("::jsonb->>'source_chunk_id'"),
        "LAW-119-2: singular SQL must not use ::jsonb on source_chunk_id extract"
    );
    assert!(
        !singular.contains("::jsonb->>'source_document_id'"),
        "LAW-119-2: singular SQL must not use ::jsonb on source_document_id extract"
    );
    assert!(
        singular.contains("->>'source_chunk_id'") && singular.contains("->>'source_document_id'"),
        "singular SQL must still filter both singular citation props"
    );
    assert!(
        singular.contains("= ANY($1::text[])"),
        "SPEC-119: singular probes must use = ANY for Index Scan (not OR+CTE IN)"
    );
    assert!(
        !singular.contains("IN (SELECT probe_id FROM probes)"),
        "SPEC-119: must not use OR+CTE IN SubPlan (Seq Scan trap on large EDGE)"
    );
}
