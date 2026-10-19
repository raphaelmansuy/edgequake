#![cfg(feature = "postgres")]
//! Query-audit regressions on real Postgres (migrations 208 and 209).
//!
//! - Facet counts aggregated in SQL equal the exact keyset sweep.
//! - Every RLS policy evaluates the session helpers once per statement.
//! - Graph list pages are served in index order, with no sort node.

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{
    facet_counts_by_scan, EdgeListFilter, GraphScanOps, NodeListFilter,
};
use edgequake_storage::{GraphStorage, PostgresAGEGraphStorage};
use sqlx::PgPool;
use uuid::Uuid;

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

/// Bare helper call: not wrapped as `( SELECT fn() AS fn)` and not qualified.
const BARE_HELPER: &str = r"(?<![\w.])current_(tenant|workspace|user)_id\(\)";
const WRAPPED_HELPER: &str = r"\( SELECT (?:public\.)?current_(?:tenant|workspace|user)_id\(\) AS current_(?:tenant|workspace|user)_id\)";

async fn graph_ready(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.eq_graph_nodes') IS NOT NULL \
         AND to_regclass('public.eq_graph_edges') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

#[tokio::test]
async fn rls_policies_evaluate_session_helpers_once_per_statement() {
    let Some(config) = require_or_skip_postgres("qaudit_rls") else {
        return;
    };
    let pool = contract_pg_pool(&config).await;
    let offenders: Vec<(String, String)> = sqlx::query_as(
        "SELECT tablename::text, policyname::text FROM pg_policies \
         WHERE schemaname = 'public' \
           AND (regexp_replace(coalesce(qual, ''), $1, '', 'g') ~ $2 \
             OR regexp_replace(coalesce(with_check, ''), $1, '', 'g') ~ $2) \
         ORDER BY 1, 2",
    )
    .bind(WRAPPED_HELPER)
    .bind(BARE_HELPER)
    .fetch_all(&pool)
    .await
    .expect("policy scan");
    assert!(
        offenders.is_empty(),
        "policies calling a session helper once per row (wrap as `(SELECT fn())`): {offenders:?}"
    );
    let wrapped: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_policies WHERE schemaname = 'public' AND qual ~* 'SELECT current_'",
    )
    .fetch_one(&pool)
    .await
    .expect("wrapped count");
    assert!(wrapped > 0, "migration 208 left no wrapped policies");
}

#[tokio::test]
async fn graph_scope_indexes_serve_ordered_pages() {
    let Some(config) = require_or_skip_postgres("qaudit_idx") else {
        return;
    };
    let pool = contract_pg_pool(&config).await;
    if !graph_ready(&pool).await {
        eprintln!("SKIP: plain graph tables unavailable");
        return;
    }
    let present: Vec<String> = sqlx::query_scalar(
        "SELECT indexname::text FROM pg_indexes WHERE schemaname = 'public' \
         AND tablename IN ('eq_graph_nodes', 'eq_graph_edges')",
    )
    .fetch_all(&pool)
    .await
    .expect("indexes");
    for wanted in [
        "idx_eq_graph_nodes_scope_node",
        "idx_eq_graph_edges_scope_pk",
    ] {
        assert!(present.iter().any(|i| i == wanted), "missing {wanted}");
    }
    for gone in [
        "idx_eq_graph_nodes_tenant_ws",
        "idx_eq_graph_edges_tenant_ws",
    ] {
        assert!(
            !present.iter().any(|i| i == gone),
            "{gone} is a strict prefix of the scope index and must stay dropped"
        );
    }

    let tenant = Uuid::new_v4().to_string();
    let workspace = Uuid::new_v4().to_string();
    // A workspace is a small slice of a busy namespace, with node ids that
    // interleave with its neighbours'. A primary-key order scan would have to
    // filter every other workspace's rows; the scope index must win instead.
    sqlx::query(
        "INSERT INTO public.eq_graph_nodes (namespace, node_id, tenant_id, workspace_id, entity_type) \
         SELECT 'qaudit_plan', 'N' || lpad(g::text, 6, '0') || '_t', $1, $2, 'T' \
         FROM generate_series(1, 3000) g ON CONFLICT DO NOTHING",
    )
    .bind(&tenant)
    .bind(&workspace)
    .execute(&pool)
    .await
    .expect("seed target workspace");
    sqlx::query(
        "INSERT INTO public.eq_graph_nodes (namespace, node_id, tenant_id, workspace_id, entity_type) \
         SELECT 'qaudit_plan', 'N' || lpad(g::text, 6, '0') || '_n' || w, $1, 'noise-ws-' || w, 'T' \
         FROM generate_series(1, 3000) g, generate_series(1, 20) w ON CONFLICT DO NOTHING",
    )
    .bind(&tenant)
    .execute(&pool)
    .await
    .expect("seed neighbouring workspaces");
    sqlx::query("ANALYZE public.eq_graph_nodes")
        .execute(&pool)
        .await
        .expect("analyze");
    // With the other plan types off, the page must still come back in index
    // order (no Sort node) from the scope index, not the pk plus a filter.
    let mut tx = pool.begin().await.expect("tx");
    sqlx::raw_sql(
        "SET LOCAL enable_seqscan = off; SET LOCAL enable_bitmapscan = off; \
         SET LOCAL enable_sort = off",
    )
    .execute(&mut *tx)
    .await
    .expect("force ordered plan");
    let plan: Vec<String> = sqlx::query_scalar(
        "EXPLAIN (COSTS OFF) SELECT node_id, properties FROM public.eq_graph_nodes n \
         WHERE n.namespace = 'qaudit_plan' AND n.tenant_id = $1 AND n.workspace_id = $2 \
         ORDER BY n.node_id OFFSET 0 LIMIT 500",
    )
    .bind(&tenant)
    .bind(&workspace)
    .fetch_all(&mut *tx)
    .await
    .expect("explain");
    tx.rollback().await.ok();
    let text = plan.join("\n");
    assert!(
        !text.contains("Sort"),
        "ordered page must not sort:\n{text}"
    );
    assert!(text.contains("idx_eq_graph_nodes_scope_node"), "{text}");
    sqlx::query("DELETE FROM public.eq_graph_nodes WHERE namespace = 'qaudit_plan'")
        .execute(&pool)
        .await
        .expect("cleanup");
}

#[tokio::test]
async fn sql_facet_counts_equal_the_exact_keyset_sweep() {
    let Some(config) = require_or_skip_postgres("qaudit_facets") else {
        return;
    };
    let pool = contract_pg_pool(&config).await;
    if !graph_ready(&pool).await {
        eprintln!("SKIP: plain graph tables unavailable");
        return;
    }
    let namespace = config.namespace.clone();
    let storage = PostgresAGEGraphStorage::new(config);
    storage.initialize().await.expect("initialize graph");

    let tenant = Uuid::new_v4().to_string();
    let workspace = Uuid::new_v4().to_string();
    let other_workspace = Uuid::new_v4().to_string();
    seed(&pool, &namespace, &tenant, &workspace, 2_300).await;
    // Rows from another workspace must not leak into the counts.
    seed(&pool, &namespace, &tenant, &other_workspace, 50).await;

    let nodes = NodeListFilter {
        tenant_id: Some(tenant.clone()),
        workspace_id: Some(workspace.clone()),
        entity_type: None,
        search: None,
        community_ids: None,
    };
    let edges = EdgeListFilter {
        tenant_id: Some(tenant),
        workspace_id: Some(workspace),
        relationship_type: None,
    };

    let sql = storage
        .facet_counts(&nodes, &edges)
        .await
        .expect("sql facets")
        .sorted();
    let sweep = facet_counts_by_scan(&storage, &nodes, &edges)
        .await
        .expect("sweep facets")
        .sorted();
    assert_eq!(sql, sweep, "SQL aggregate must equal the keyset sweep");

    let total_nodes: usize = sql.entity_types.iter().map(|(_, n)| n).sum();
    assert_eq!(total_nodes, 2_300, "counts cover the whole workspace");
    assert!(
        sql.entity_types.iter().any(|(k, _)| k == "UNKNOWN"),
        "typeless nodes land in UNKNOWN"
    );
    assert!(
        sql.relationship_types
            .iter()
            .any(|(k, _)| k == "RELATED_TO"),
        "untyped edges land in RELATED_TO"
    );
    assert!(sql.communities.iter().any(|(k, _)| k == "7"), "integer ids");
    assert!(
        sql.communities.iter().any(|(k, _)| k == "c-str"),
        "string ids"
    );
}

#[tokio::test]
async fn fuzzy_label_search_scope_comes_from_the_session_not_the_arguments() {
    let Some(config) = require_or_skip_postgres("qaudit_fuzzy") else {
        return;
    };
    let pool = contract_pg_pool(&config).await;
    if !graph_ready(&pool).await {
        eprintln!("SKIP: plain graph tables unavailable");
        return;
    }
    let function_present: bool = sqlx::query_scalar(
        "SELECT to_regprocedure('public.eq_graph_search_labels_fuzzy(text,text,integer,text,text)') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("function probe");
    assert!(
        function_present,
        "migration 210 must create the owner function"
    );

    let namespace = config.namespace.clone();
    let (tenant_a, ws_a) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let (tenant_b, ws_b) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    for (tenant, ws, names) in [
        (
            &tenant_a,
            &ws_a,
            ["lymphoma staging a", "lymphoma grading a"],
        ),
        (
            &tenant_b,
            &ws_b,
            ["lymphoma therapy b", "lymphoma outcome b"],
        ),
    ] {
        for name in names {
            sqlx::query(
                "INSERT INTO public.eq_graph_nodes (namespace, node_id, tenant_id, workspace_id) \
                 VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            )
            .bind(&namespace)
            .bind(format!("{name} {}", &ws[..8]))
            .bind(tenant)
            .bind(ws)
            .execute(&pool)
            .await
            .expect("seed node");
        }
    }

    // Session A, but the caller passes workspace B's ids as arguments.
    let hostile = fuzzy_as_session(&pool, &namespace, (&tenant_a, &ws_a), (&tenant_b, &ws_b)).await;
    assert_eq!(
        hostile.len(),
        2,
        "session A sees its own two nodes: {hostile:?}"
    );
    assert!(
        hostile.iter().all(|l| l.ends_with(&ws_a[..8])),
        "arguments must not widen the scope: {hostile:?}"
    );
    // Session B passing A's ids sees only B.
    let other = fuzzy_as_session(&pool, &namespace, (&tenant_b, &ws_b), (&tenant_a, &ws_a)).await;
    assert!(other.iter().all(|l| l.ends_with(&ws_b[..8])), "{other:?}");
}

/// Call the owner function as `edgequake_tenant_access` with a session scope
/// and (possibly different) scope arguments.
async fn fuzzy_as_session(
    pool: &PgPool,
    namespace: &str,
    session: (&str, &str),
    args: (&str, &str),
) -> Vec<String> {
    let mut tx = pool.begin().await.expect("tx");
    sqlx::query("SELECT set_config('app.current_tenant_id', $1, true), set_config('app.current_workspace_id', $2, true)")
        .bind(session.0)
        .bind(session.1)
        .execute(&mut *tx)
        .await
        .expect("session scope");
    sqlx::raw_sql("SET LOCAL ROLE edgequake_tenant_access")
        .execute(&mut *tx)
        .await
        .expect("tenant role");
    let labels: Vec<String> = sqlx::query_scalar(
        "SELECT label FROM public.eq_graph_search_labels_fuzzy($1, 'lymphoma', 10, $2, $3) ORDER BY 1",
    )
    .bind(namespace)
    .bind(args.0)
    .bind(args.1)
    .fetch_all(&mut *tx)
    .await
    .expect("fuzzy search");
    tx.rollback().await.ok();
    labels
}

/// `count` nodes cycling through types/communities, plus a chain of edges.
async fn seed(pool: &PgPool, namespace: &str, tenant: &str, workspace: &str, count: i32) {
    sqlx::query(
        "INSERT INTO public.eq_graph_nodes \
             (namespace, node_id, tenant_id, workspace_id, entity_type, properties) \
         SELECT $1, $4 || '_N' || lpad(g::text, 6, '0'), $2, $3, t.et, \
                CASE g % 5 \
                    WHEN 0 THEN '{}'::jsonb \
                    WHEN 1 THEN '{\"community_id\": 7}'::jsonb \
                    WHEN 2 THEN '{\"community_id\": \"c-str\"}'::jsonb \
                    WHEN 3 THEN '{\"community_id\": 8}'::jsonb \
                    ELSE '{\"community_id\": null}'::jsonb END \
                || CASE WHEN t.et IS NULL THEN '{}'::jsonb \
                        ELSE jsonb_build_object('entity_type', t.et) END \
         FROM generate_series(1, $5) g, \
              LATERAL (SELECT CASE g % 4 WHEN 0 THEN NULL WHEN 1 THEN 'PERSON' \
                                         WHEN 2 THEN 'ORG' ELSE 'EVENT' END AS et) t \
         ON CONFLICT DO NOTHING",
    )
    .bind(namespace)
    .bind(tenant)
    .bind(workspace)
    .bind(workspace)
    .bind(count)
    .execute(pool)
    .await
    .expect("seed nodes");
    sqlx::query(
        "INSERT INTO public.eq_graph_edges \
             (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, properties) \
         SELECT $1, $4 || '_N' || lpad(g::text, 6, '0'), $4 || '_N' || lpad((g + 1)::text, 6, '0'), \
                CASE g % 3 WHEN 0 THEN '' WHEN 1 THEN 'USES' ELSE 'OWNS' END, $2, $3, \
                CASE g % 3 WHEN 0 THEN '{}'::jsonb \
                           WHEN 1 THEN '{\"relation_type\": \"USES\"}'::jsonb \
                           ELSE '{\"relation_type\": \"OWNS\"}'::jsonb END \
         FROM generate_series(1, $5) g ON CONFLICT DO NOTHING",
    )
    .bind(namespace)
    .bind(tenant)
    .bind(workspace)
    .bind(workspace)
    .bind(count - 1)
    .execute(pool)
    .await
    .expect("seed edges");
}
