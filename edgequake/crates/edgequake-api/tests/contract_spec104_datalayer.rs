//! SPEC-104 — Storage inspector / tenant slug contracts (source + optional PG).
//!
//! Unit/source gates always run. Postgres cases skip unless `DATABASE_URL` is set.

#![cfg(feature = "postgres")]

use edgequake_api::storage_inspector::{InspectorConfig, StorageInspector};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn e2e_104_01_source_uses_workspace_id_not_id() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("WHERE workspace_id::text = $1"),
        "INV-D2 must probe workspaces.workspace_id"
    );
    assert!(
        !src.contains("WHERE id::text = $1"),
        "INV-D2 must not use workspaces.id (42703)"
    );
    assert!(
        !src.contains("unwrap_or(true)"),
        "INV-D2 must not fail-open with unwrap_or(true)"
    );
}

/// E2E-107-03 source gate: INV-03 repair arm must exist (LogOnly / residual ops).
#[test]
fn e2e_107_03_source_inv03_logonly_repair() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("repair_recommendation_for_invariant"),
        "INV repair mapping must be a testable helper"
    );
    assert!(
        src.contains("\"INV-03\""),
        "INV-03 must have an explicit repair arm (SPEC-107)"
    );
    assert!(
        src.contains("no SAFE auto-repair"),
        "INV-03 LogOnly message must forbid SAFE auto-mutate"
    );
}

/// E2E-107-R2-01: INV-C must chunk by SOURCE_PREFIX_BATCH_LIMIT (LAW-H1).
#[test]
fn e2e_107_r2_inv_c_chunks_by_batch_limit() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("SOURCE_PREFIX_BATCH_LIMIT"),
        "INV-C must use public SPEC-089 batch SSOT"
    );
    assert!(
        src.contains("SOURCE_COUNT_STATEMENT_TIMEOUT_MS"),
        "INV-C must use public SPEC-089 timeout SSOT"
    );
    assert!(
        src.contains("inv_c_gin_node_counts_one_batch"),
        "INV-C must split into one_batch round-trips"
    );
    let batches = include_str!("../src/storage_inspector/inv_c_batches.rs");
    assert!(
        batches.contains("let mut limit = batch_limit.max(1);")
            && batches.contains("&prefixes[cursor..(cursor + limit).min(prefixes.len())]"),
        "INV-C must chunk prefixes at SOURCE_PREFIX_BATCH_LIMIT (not one-shot ≤50)"
    );
    assert!(
        batches.contains("\"57014\"") && batches.contains("limit = batch.len() / 2;"),
        "INV-C must halve only statement-timeout batches (SPEC-149)"
    );
    assert!(
        src.contains("edgequake_storage::node_counts_by_source_prefixes_sql(")
            && edgequake_storage::node_counts_by_source_prefixes_sql("g")
                .contains("DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES"),
        "INV-C SQL must keep the shared dataop marker"
    );
    let read = include_str!("../src/document_read_model.rs");
    assert!(
        read.contains("DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES"),
        "list reconcile soft-fail must tag the dataop for 57014 greps"
    );
}

#[test]
fn e2e_104_02_source_graph_not_hardcoded_edgequake() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("for_namespace"),
        "InspectorConfig must expose for_namespace SSOT helper"
    );
    assert!(
        !src.contains("graph_name: \"edgequake\".to_string()"),
        "must not hardcode graph_name edgequake in Default"
    );
    let cfg = InspectorConfig::default();
    assert_eq!(cfg.graph_name, "eq_eq_default_graph");
}

#[test]
fn e2e_104_03_source_inv03_dual_presence() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("FROM public.chunks c WHERE c.document_id = d.id"),
        "INV-03 must check public.chunks"
    );
    assert!(
        src.contains("k.key LIKE d.id::text || '-chunk-%'"),
        "INV-03 harden must dual-read KV when present (EC-16)"
    );
    assert!(
        src.contains("IN ('indexed', 'completed')"),
        "INV-03 must cover terminal indexed|completed (SPEC-107)"
    );
    assert!(
        src.contains("INV-C: batched GIN entity count failed — skipping"),
        "INV-C GIN failure must stay log-visible (SPEC-107 LAW-I2)"
    );
    assert!(
        src.contains("\"inv_c_skipped\""),
        "INV-C skip must emit Info-level inv_c_skipped (visible, not drift)"
    );
    assert!(
        src.contains("Severity::Info"),
        "INV-C skip must use Severity::Info so has_warning stays false"
    );
    assert!(
        !src.contains("\"inv_c_gin_batch\""),
        "a timed-out INV-C sample must not use the old Warning inv_c_gin_batch name"
    );
    assert!(
        src.contains("chunk_embeddings"),
        "INV-01 must prefer typed chunk_embeddings (EC-18)"
    );
    assert!(
        src.contains("require_safe_sql_ident"),
        "A+: dynamic table names must pass identifier allowlist"
    );
    assert!(
        src.contains("column_name='document_id'"),
        "A+: INV-01 legacy path must prefer document_id column when present"
    );
}

#[test]
fn e2e_384_source_inv07_inflight_without_task() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("check_inv07_inflight_docs_without_task"),
        "INV-07 must be a real inspector method, not docs-only"
    );
    assert!(
        src.contains("\"INV-07\""),
        "INV-07 must have an explicit repair arm"
    );
    assert!(
        src.contains("no SAFE auto-enqueue from inspector"),
        "INV-07 LogOnly must forbid inspector enqueue"
    );
    assert!(
        src.contains("inflight_orphan_minutes"),
        "INV-07 must age-filter past the early-admit window"
    );
    assert!(
        src.contains("object_kind = 'document_batch'"),
        "INV-07 must exclude docs that already have a document_batch projection event"
    );
}

#[test]
fn one_serving_writer_list_stays_read_only() {
    let list = include_str!("../src/services/list_run_enrich.rs");
    assert!(
        !list.contains("open_serving_fence_when_deliveries_settled"),
        "GET /documents must not mutate chunk_serving_state"
    );
    assert!(
        list.contains("chunk_serving_state"),
        "list must still read chunk_serving_state for query_ready"
    );
    let reconcile = include_str!("../src/services/pending_doc_task_reconcile.rs");
    assert!(
        reconcile.contains("open_settled_serving_fences_bounded"),
        "reconcile must use one bounded fence open"
    );
    assert!(
        !reconcile.contains("try_heal_serving_fence_indexed"),
        "per-document fence heal must be removed"
    );
}

#[test]
fn e2e_104_04_source_tenant_atomic_upsert() {
    let src = include_str!("../../edgequake-core/src/workspace_service_impl/tenant_ops.rs");
    assert!(
        src.contains("ON CONFLICT (slug) DO UPDATE"),
        "tenant create must use atomic ON CONFLICT DO UPDATE RETURNING"
    );
    assert!(
        src.contains("RETURNING tenant_id, name, slug"),
        "upsert must RETURNING the row"
    );
    assert!(
        src.contains("Error::conflict"),
        "A+: identity clash must be Error::Conflict at service layer (EC-11)"
    );
    let api_err = include_str!("../src/error.rs");
    assert!(
        api_err.contains("CoreError::Conflict(msg) => ApiError::Conflict(msg)"),
        "CoreError::Conflict must map to HTTP 409"
    );
}

#[test]
fn e2e_104_05_source_gin_check_all_graphs() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("idx_node_source_ids_gin"),
        "inspector must surface missing M038 GIN (LAW-I4)"
    );
    assert!(
        src.contains("check_all_graphs_node_source_ids_gin")
            || src.contains("check_node_source_ids_gin_for"),
        "schema layer must scan graphs for GIN"
    );
    assert!(
        src.contains("ag_catalog.ag_graph"),
        "must discover eq_*_graph from ag_catalog"
    );
}

/// SPEC-149: INV-C must count with the storage SSOT (both lineage GIN arms),
/// never a hand-copied CTE that can drift from `analytics_ops`.
#[test]
fn e2e_149_inv_c_uses_storage_count_sql_ssot() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("edgequake_storage::node_counts_by_source_prefixes_sql("),
        "INV-C must build its count SQL from edgequake_storage"
    );
    assert!(
        !src.contains("-> 'source_ids')") && !src.contains("-> 'source_chunk_ids')"),
        "INV-C must not inline lineage GIN predicates"
    );
    let sql = edgequake_storage::node_counts_by_source_prefixes_sql("g");
    for key in edgequake_storage::INDEXED_LINEAGE_ARRAY_KEYS {
        assert!(
            sql.contains(&format!("-> '{key}') @>")),
            "shared count SQL must GIN-probe {key}: {sql}"
        );
    }
}

#[test]
fn e2e_104_06_naming_helpers_match_inspector() {
    let storage = edgequake_storage::PostgresConfig {
        namespace: "default".into(),
        ..Default::default()
    };
    let cfg = InspectorConfig::for_namespace("default");
    assert_eq!(cfg.graph_name, storage.age_graph_name());
    assert_eq!(cfg.kv_table, storage.bare_kv_table());
    assert_eq!(cfg.vector_table, storage.bare_vectors_table());

    let ws = edgequake_storage::PostgresConfig {
        namespace: "my-ws".into(),
        ..Default::default()
    };
    let cfg2 = InspectorConfig::for_namespace("my-ws");
    assert_eq!(cfg2.graph_name, ws.age_graph_name());
    assert_eq!(cfg2.kv_table, ws.bare_kv_table());
}

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty())
}

#[tokio::test]
async fn e2e_104_pg_inspector_inv03_orphan_fires() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_104_pg_inspector_inv03_orphan_fires: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    let cfg = InspectorConfig::for_namespace("default");
    let inspector = StorageInspector::new(Arc::new(pool.clone()), cfg);

    let ws: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT workspace_id, tenant_id FROM workspaces ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let Some((workspace_id, tenant_id)) = ws else {
        eprintln!("SKIP: no workspace row");
        return;
    };

    let doc_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
    let _ = sqlx::query(
        r#"
        INSERT INTO documents (id, workspace_id, tenant_id, title, content, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'spec104', 'body', 'indexed', NOW(), NOW())
        ON CONFLICT (id) DO UPDATE SET status = 'indexed'
        "#,
    )
    .bind(doc_id)
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(&pool)
    .await;

    let _ = sqlx::query("DELETE FROM chunks WHERE document_id = $1")
        .bind(doc_id)
        .execute(&pool)
        .await;

    // Ensure no KV chunk keys either (true orphan — E2E-104-08).
    let kv = InspectorConfig::default().kv_table;
    let kv_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)",
    )
    .bind(&kv)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if kv_exists {
        let del = format!("DELETE FROM {kv} WHERE key LIKE $1");
        let _ = sqlx::query(&del)
            .bind(format!("{doc_id}-chunk-%"))
            .execute(&pool)
            .await;
    }

    let report2 = inspector.inspect().await;
    let inv03 = report2
        .invariant_violations
        .iter()
        .find(|v| v.invariant_id == "INV-03");
    assert!(
        inv03.is_some(),
        "E2E-104-08: expected INV-03 for true orphan; report={report2:?}"
    );

    let _ = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(&pool)
        .await;
}

#[tokio::test]
async fn e2e_384_pg_inspector_inv07_aged_orphan_fires() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_384_pg_inspector_inv07_aged_orphan_fires: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    let cfg = InspectorConfig {
        inflight_orphan_minutes: 15,
        ..InspectorConfig::for_namespace("default")
    };
    let inspector = StorageInspector::new(Arc::new(pool.clone()), cfg);

    let ws: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT workspace_id, tenant_id FROM workspaces ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    let Some((workspace_id, tenant_id)) = ws else {
        eprintln!("SKIP: no workspace row");
        return;
    };

    let aged_id = Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").unwrap();
    let fresh_id = Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap();
    let test_id_text = [
        "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
        "ffffffff-ffff-ffff-ffff-ffffffffffff",
    ];

    // BEFORE UPDATE trigger forces updated_at=NOW(), so never UPDATE the seed:
    // delete then INSERT. Epoch timestamp wins ORDER BY aged_at ASC vs fleet orphans.
    let _ = sqlx::query("DELETE FROM chunks WHERE document_id = ANY($1::uuid[])")
        .bind([aged_id, fresh_id])
        .execute(&pool)
        .await;
    sqlx::query("DELETE FROM documents WHERE id = ANY($1::uuid[])")
        .bind([aged_id, fresh_id])
        .execute(&pool)
        .await
        .expect("cleanup spec384 seed documents");

    let kv = InspectorConfig::default().kv_table;
    let kv_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)",
    )
    .bind(&kv)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if kv_exists {
        let del = format!("DELETE FROM {kv} WHERE key = ANY($1::text[])");
        let keys: Vec<String> = test_id_text
            .iter()
            .map(|id| format!("{id}-metadata"))
            .collect();
        let _ = sqlx::query(&del).bind(keys).execute(&pool).await;
    }

    let _ = sqlx::query(
        "DELETE FROM tasks WHERE document_id = ANY($1::text[]) \
         OR payload->'task_data'->>'document_id' = ANY($1::text[]) \
         OR payload->'task_data'->>'existing_document_id' = ANY($1::text[])",
    )
    .bind(test_id_text)
    .execute(&pool)
    .await;

    sqlx::query(
        r#"
        INSERT INTO documents (id, workspace_id, tenant_id, title, content, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'spec384-aged', '', 'processing', TIMESTAMPTZ '1970-01-01', TIMESTAMPTZ '1970-01-01')
        "#,
    )
    .bind(aged_id)
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("insert aged INV-07 seed");

    sqlx::query(
        r#"
        INSERT INTO documents (id, workspace_id, tenant_id, title, content, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'spec384-fresh', '', 'processing', NOW(), NOW())
        "#,
    )
    .bind(fresh_id)
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("insert fresh INV-07 seed");

    let report = inspector.inspect().await;
    let inv07 = report
        .invariant_violations
        .iter()
        .find(|v| v.invariant_id == "INV-07");
    assert!(
        inv07.is_some(),
        "expected INV-07 for aged processing orphan; report={report:?}"
    );
    let samples = &inv07.unwrap().sample_ids;
    assert!(
        samples
            .iter()
            .any(|s| s == "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee"),
        "aged orphan must be sampled: {samples:?}"
    );
    assert!(
        !samples
            .iter()
            .any(|s| s == "ffffffff-ffff-ffff-ffff-ffffffffffff"),
        "fresh early-admit window must not fire INV-07: {samples:?}"
    );

    let _ = sqlx::query("DELETE FROM documents WHERE id = ANY($1::uuid[])")
        .bind([aged_id, fresh_id])
        .execute(&pool)
        .await;
}

#[tokio::test]
async fn e2e_104_07_pg_inv03_clear_when_kv_chunk_present() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_104_07_pg_inv03_clear_when_kv_chunk_present: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    let kv = InspectorConfig::default().kv_table;
    let kv_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)",
    )
    .bind(&kv)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !kv_exists {
        eprintln!("SKIP: no KV table (post-125) — dual-read path N/A");
        return;
    }

    let ws: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT workspace_id, tenant_id FROM workspaces ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    let Some((workspace_id, tenant_id)) = ws else {
        eprintln!("SKIP: no workspace");
        return;
    };

    let doc_id = Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
    let _ = sqlx::query(
        r#"
        INSERT INTO documents (id, workspace_id, tenant_id, title, content, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'spec104-kv', 'body', 'indexed', NOW(), NOW())
        ON CONFLICT (id) DO UPDATE SET status = 'indexed'
        "#,
    )
    .bind(doc_id)
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(&pool)
    .await;

    let _ = sqlx::query("DELETE FROM chunks WHERE document_id = $1")
        .bind(doc_id)
        .execute(&pool)
        .await;

    let key = format!("{doc_id}-chunk-0");
    let upsert = format!(
        "INSERT INTO {kv} (key, value) VALUES ($1, '{{}}'::jsonb) ON CONFLICT (key) DO NOTHING"
    );
    // Some KV schemas use different columns — try best-effort.
    if sqlx::query(&upsert)
        .bind(&key)
        .execute(&pool)
        .await
        .is_err()
    {
        let upsert2 = format!(
            "INSERT INTO {kv} (key, value, created_at) VALUES ($1, '{{}}'::jsonb, NOW()) ON CONFLICT (key) DO NOTHING"
        );
        let _ = sqlx::query(&upsert2).bind(&key).execute(&pool).await;
    }

    let inspector = StorageInspector::new(
        Arc::new(pool.clone()),
        InspectorConfig::for_namespace("default"),
    );
    let report = inspector.inspect().await;
    let hits: Vec<_> = report
        .invariant_violations
        .iter()
        .filter(|v| v.invariant_id == "INV-03")
        .filter(|v| v.sample_ids.iter().any(|s| s == &doc_id.to_string()))
        .collect();
    assert!(
        hits.is_empty(),
        "E2E-104-07: KV chunk key must clear INV-03 for this doc; report={report:?}"
    );

    let _ = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query(&format!("DELETE FROM {kv} WHERE key = $1"))
        .bind(&key)
        .execute(&pool)
        .await;
}

#[tokio::test]
async fn e2e_104_09_pg_tenant_same_name_ok_diff_name_detectable() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_104_09: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    use edgequake_core::WorkspaceService;
    let svc = edgequake_core::WorkspaceServiceImpl::new(pool.clone());
    let slug = format!("spec104-{}", &Uuid::new_v4().to_string()[..8]);
    let t1 = edgequake_core::Tenant::new("SPEC-104 Same", &slug);
    let id1 = t1.tenant_id;
    let created = svc.create_tenant(t1).await.expect("create");
    assert_eq!(created.tenant_id, id1);

    // Same name → existing id (handler would 200).
    let t_same = edgequake_core::Tenant::new("SPEC-104 Same", &slug);
    let again = svc.create_tenant(t_same).await.expect("idempotent");
    assert_eq!(again.tenant_id, id1);
    assert_eq!(again.name, "SPEC-104 Same");

    // Different name → Conflict at service layer (A+; HTTP maps to 409).
    let t_diff = edgequake_core::Tenant::new("Other Org", &slug);
    let err = svc
        .create_tenant(t_diff)
        .await
        .expect_err("expected Conflict on identity clash");
    assert!(
        err.to_string().contains("already exists"),
        "conflict message: {err}"
    );

    let _ = svc.delete_tenant(id1).await;
}

#[tokio::test]
async fn e2e_104_10_pg_gin_missing_on_extra_graph() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_104_10: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    // Create a disposable AGE graph without GIN if AGE is available.
    let graph = "eq_eq_spec104tmp_graph";
    let created = sqlx::query(&format!("SELECT * FROM ag_catalog.create_graph('{graph}')"))
        .execute(&pool)
        .await;
    if let Err(e) = &created {
        // Fleet AGE may refuse create_graph (e.g. missing graphid_ops) — cannot
        // prove multi-graph GIN absence without a second graph. Soft-skip.
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name::text = $1)",
        )
        .bind(graph)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);
        if !exists {
            eprintln!("SKIP e2e_104_10: create_graph failed and graph absent: {e}");
            return;
        }
    }
    let _ = sqlx::query(&format!(
        "DROP INDEX IF EXISTS {graph}.idx_node_source_ids_gin"
    ))
    .execute(&pool)
    .await;

    let inspector = StorageInspector::new(
        Arc::new(pool.clone()),
        InspectorConfig::for_namespace("default"),
    );
    let report = inspector.inspect().await;
    let gin_hit = report.schema_issues.iter().any(|i| {
        i.check_name.contains("m038_idx_node_source_ids_gin") && i.check_name.contains(graph)
            || i.description.contains(graph)
    });
    assert!(
        gin_hit,
        "E2E-104-10: expected GIN warning for {graph}; issues={:?}",
        report.schema_issues
    );

    let _ = sqlx::query(&format!(
        "SELECT * FROM ag_catalog.drop_graph('{graph}', true)"
    ))
    .execute(&pool)
    .await;
}

#[tokio::test]
async fn e2e_104_pg_tenant_slug_get_or_create() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_104_pg_tenant_slug_get_or_create: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    use edgequake_core::WorkspaceService;
    let svc = edgequake_core::WorkspaceServiceImpl::new(pool.clone());
    let slug = format!("spec104-{}", &Uuid::new_v4().to_string()[..8]);
    let t1 = edgequake_core::Tenant::new("SPEC-104 Tenant", &slug);
    let id1 = t1.tenant_id;
    let created = svc.create_tenant(t1).await.expect("create");
    assert_eq!(created.tenant_id, id1);

    let t2 = edgequake_core::Tenant::new("SPEC-104 Tenant", &slug);
    let existing = svc.create_tenant(t2).await.expect("get-or-create");
    assert_eq!(existing.tenant_id, id1);

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tenants WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 1);

    let _ = svc.delete_tenant(id1).await;
}
