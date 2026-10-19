//! SPEC-164: every plain-graph query through `GraphStorage` on real Postgres.
//!
//! Greenfield database (no Apache AGE). Migration 170 marks the ledger ready.
//! Timings are written to `/tmp/spec164-measurements.json`.

use std::collections::HashMap;
use std::time::Instant;

use edgequake_storage::traits::{EdgeListFilter, GraphScanOps, NodeListFilter};
use edgequake_storage::{
    GraphPropertyWriteMode, GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps,
    GraphStorageReadOps, PostgresAGEGraphStorage, PostgresConfig,
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

const M090: &str = include_str!("../../../migrations/090_eq_merge_graph_properties.sql");
const M170: &str = include_str!("../../../migrations/170_spec164_plain_graph.sql");
const M171: &str = include_str!("../../../migrations/171_spec164_drop_age.sql");
const M204: &str = include_str!("../../../migrations/204_graph_label_search_rls.sql");
const M206: &str = include_str!("../../../migrations/206_rls_leakproof_predicates.sql");

macro_rules! measure {
    ($out:ident, $name:expr, $fut:expr) => {{
        let start = Instant::now();
        let _value = $fut.await;
        $out.insert($name.into(), json!(start.elapsed().as_secs_f64() * 1000.0));
    }};
}

#[tokio::test]
#[allow(deprecated)]
async fn plain_graph_queries_are_proven_and_measured() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP spec164 e2e: set DATABASE_URL to a Postgres that can CREATE DATABASE");
        return;
    };
    let db = format!("spec164_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin)
        .await
        .expect("admin connect");
    sqlx::query(&format!("CREATE DATABASE {db}"))
        .execute(&admin_pool)
        .await
        .expect("create database");
    admin_pool.close().await;

    let url = replace_db(&admin, &db);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("db connect");
    sqlx::raw_sql(STUBS)
        .execute(&pool)
        .await
        .expect("tenant stubs");
    sqlx::raw_sql(M090).execute(&pool).await.expect("090");
    sqlx::raw_sql(M170).execute(&pool).await.expect("170");
    // search_labels and search_nodes call the owner functions from 204 and 206.
    // 206 also alters chunks, entities, and documents, which this graph fixture
    // does not create, so only the search function is applied.
    sqlx::raw_sql(M204).execute(&pool).await.expect("204");
    let marker = "CREATE OR REPLACE FUNCTION public.eq_graph_search_nodes";
    let search_fn = M206
        .split_once(marker)
        .map(|(_, rest)| format!("{marker}{rest}"))
        .expect("206 search function");
    sqlx::raw_sql(&search_fn)
        .execute(&pool)
        .await
        .expect("206 search function");

    let (host, port, user, password) = parse_url(&url);
    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();
    // Scoped graph reads confirm the tenant owns the workspace before they
    // query (`plain::scan`), so the fixture registers both workspaces.
    for (workspace, tenant) in [(ws_a, tenant_a), (ws_b, tenant_b)] {
        sqlx::query("INSERT INTO public.workspaces (workspace_id, tenant_id) VALUES ($1, $2)")
            .bind(workspace)
            .bind(tenant)
            .execute(&pool)
            .await
            .expect("register workspace");
    }
    let config = PostgresConfig::new(host, port, &db, user, password)
        .with_namespace("spec164")
        .with_max_connections(4);
    let storage = PostgresAGEGraphStorage::new(config);
    storage.initialize().await.expect("initialize plain graph");
    let age_installed: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age probe");
    assert!(
        !age_installed,
        "this e2e proves the no-AGE path; point DATABASE_URL at Postgres with pgvector and without Apache AGE"
    );
    let trgm_schema: String = sqlx::query_scalar(
        "SELECT n.nspname::text FROM pg_extension e
         JOIN pg_namespace n ON n.oid = e.extnamespace
         WHERE e.extname = 'pg_trgm'",
    )
    .fetch_one(&pool)
    .await
    .expect("pg_trgm");
    assert_eq!(
        trgm_schema, "public",
        "pg_trgm must install into public with no AGE catalog"
    );

    let alice = node(
        "ALICE",
        &tenant_a,
        &ws_a,
        "Person",
        "alice example",
        &["doc1-chunk-0"],
    );
    let bob = node(
        "BOB",
        &tenant_a,
        &ws_a,
        "Person",
        "bob example",
        &["doc1-chunk-1"],
    );
    let carol = node(
        "CAROL",
        &tenant_a,
        &ws_a,
        "Org",
        "carol example",
        &["doc1-chunk-0"],
    );
    let dave = node(
        "DAVE",
        &tenant_b,
        &ws_b,
        "Person",
        "dave hidden",
        &["doc2-chunk-0"],
    );
    storage
        .upsert_nodes_batch(&[alice, bob, carol, dave])
        .await
        .expect("upsert nodes");
    storage
        .upsert_edges_batch(&[
            edge("ALICE", "BOB", "KNOWS", &tenant_a, &ws_a, "rel-alice-bob"),
            edge(
                "BOB",
                "CAROL",
                "WORKS_WITH",
                &tenant_a,
                &ws_a,
                "rel-bob-carol",
            ),
        ])
        .await
        .expect("upsert edges");

    let mut measurements = serde_json::Map::new();
    let tid = tenant_a.to_string();
    let wid = ws_a.to_string();
    let node_filter = NodeListFilter {
        tenant_id: Some(tid.clone()),
        workspace_id: Some(wid.clone()),
        ..Default::default()
    };
    let edge_filter = EdgeListFilter {
        tenant_id: Some(tid.clone()),
        workspace_id: Some(wid.clone()),
        relationship_type: None,
    };

    measure!(measurements, "get_node", storage.get_node("ALICE"));
    let got = storage
        .get_node("ALICE")
        .await
        .expect("get")
        .expect("alice");
    assert_eq!(got.id, "ALICE");
    assert!(storage.has_node("ALICE").await.expect("has"));
    let degree = storage.node_degree("ALICE").await.expect("degree");
    assert_eq!(degree, 1);
    assert_eq!(storage.node_degree("BOB").await.expect("bob degree"), 2);
    storage
        .upsert_edges_batch(&[edge(
            "ALICE",
            "BOB",
            "KNOWS",
            &tenant_a,
            &ws_a,
            "rel-alice-bob",
        )])
        .await
        .expect("reupsert edge");
    assert_eq!(
        storage.node_degree("ALICE").await.expect("degree stable"),
        1,
        "conflict update must not double-count degree"
    );
    storage
        .upsert_edges_batch(&[edge(
            "ALICE",
            "ALICE",
            "SELF",
            &tenant_a,
            &ws_a,
            "rel-alice-self",
        )])
        .await
        .expect("self loop");
    assert_eq!(
        storage
            .node_degree("ALICE")
            .await
            .expect("self loop degree"),
        3,
        "a self-loop counts at both ends"
    );
    storage
        .delete_edge("ALICE", "ALICE")
        .await
        .expect("drop self loop");
    assert_eq!(storage.node_degree("ALICE").await.expect("unloop"), 1);
    measure!(measurements, "node_degree", storage.node_degree("ALICE"));

    let degrees = storage
        .node_degrees_batch(&["ALICE".into(), "BOB".into()])
        .await
        .expect("degrees");
    assert!(degrees.iter().any(|(id, d)| id == "ALICE" && *d == 1));
    measure!(
        measurements,
        "node_degrees_batch",
        storage.node_degrees_batch(&["ALICE".into(), "BOB".into()])
    );

    let all = storage.get_all_nodes().await.expect("all nodes");
    assert!(all.iter().any(|n| n.id == "CAROL"));
    measure!(measurements, "get_all_nodes", storage.get_all_nodes());

    let batch = storage
        .get_nodes_batch(&["BOB".into()])
        .await
        .expect("batch");
    assert!(batch.contains_key("BOB"));
    measure!(
        measurements,
        "get_nodes_batch",
        storage.get_nodes_batch(&["BOB".into()])
    );

    let with_deg = storage
        .get_nodes_with_degrees_batch(&["BOB".into()])
        .await
        .expect("with deg");
    assert_eq!(with_deg.len(), 1);
    assert_eq!(with_deg[0].1, 1, "BOB in-degree");
    assert_eq!(with_deg[0].2, 1, "BOB out-degree");
    measure!(
        measurements,
        "get_nodes_with_degrees_batch",
        storage.get_nodes_with_degrees_batch(&["BOB".into()])
    );

    let edge_row = storage
        .get_edge("ALICE", "BOB")
        .await
        .expect("edge")
        .expect("knows");
    assert_eq!(edge_row.source, "ALICE");
    measure!(measurements, "get_edge", storage.get_edge("ALICE", "BOB"));

    let incident = storage.get_node_edges("BOB").await.expect("incident");
    assert_eq!(incident.len(), 2);
    measure!(
        measurements,
        "get_node_edges",
        storage.get_node_edges("BOB")
    );

    let incident_batch = storage
        .get_incident_edges_batch(&["ALICE".into()], Some(&tid), Some(&wid))
        .await
        .expect("incident batch");
    assert!(!incident_batch.is_empty());
    measure!(
        measurements,
        "get_incident_edges_batch",
        storage.get_incident_edges_batch(&["ALICE".into()], Some(&tid), Some(&wid))
    );

    let all_edges = storage.get_all_edges().await.expect("all edges");
    assert_eq!(all_edges.len(), 2);
    measure!(measurements, "get_all_edges", storage.get_all_edges());

    let for_nodes = storage
        .get_edges_for_nodes_batch(&["ALICE".into(), "BOB".into()])
        .await
        .expect("edges for nodes");
    assert!(!for_nodes.is_empty());
    measure!(
        measurements,
        "get_edges_for_nodes_batch",
        storage.get_edges_for_nodes_batch(&["ALICE".into(), "BOB".into()])
    );

    let for_set = storage
        .get_edges_for_node_set(
            &["ALICE".into(), "BOB".into(), "CAROL".into()],
            Some(&tid),
            Some(&wid),
        )
        .await
        .expect("edges for set");
    assert_eq!(for_set.len(), 2);
    measure!(
        measurements,
        "get_edges_for_node_set",
        storage.get_edges_for_node_set(
            &["ALICE".into(), "BOB".into(), "CAROL".into()],
            Some(&tid),
            Some(&wid),
        )
    );

    let neighbors = storage
        .get_neighbors("ALICE", 2, Some(&tid), Some(&wid))
        .await
        .expect("neighbors");
    let ids: Vec<_> = neighbors.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"BOB") && ids.contains(&"CAROL"), "{ids:?}");
    assert!(!ids.contains(&"DAVE"));
    measure!(
        measurements,
        "get_neighbors",
        storage.get_neighbors("ALICE", 2, Some(&tid), Some(&wid))
    );

    let kg = storage
        .get_knowledge_graph("ALICE", 2, 50, Some(&tid), Some(&wid))
        .await
        .expect("kg");
    assert!(kg.nodes.iter().any(|n| n.id == "CAROL"));
    measure!(
        measurements,
        "get_knowledge_graph",
        storage.get_knowledge_graph("ALICE", 2, 50, Some(&tid), Some(&wid))
    );

    let found = storage
        .search_nodes("alice", 10, Some("Person"), Some(&tid), Some(&wid))
        .await
        .expect("search nodes");
    assert!(found.iter().any(|(n, _)| n.id == "ALICE"));
    measure!(
        measurements,
        "search_nodes",
        storage.search_nodes("alice", 10, Some("Person"), Some(&tid), Some(&wid))
    );

    let labels = storage
        .search_labels("person", 10, Some(&tid), Some(&wid))
        .await
        .expect("search labels");
    assert!(labels
        .iter()
        .any(|l| l.eq_ignore_ascii_case("person") || l == "ALICE"));
    measure!(
        measurements,
        "search_labels",
        storage.search_labels("person", 10, Some(&tid), Some(&wid))
    );

    let popular = storage
        .get_popular_labels(5, Some(&tid), Some(&wid))
        .await
        .expect("popular labels");
    assert!(popular.iter().any(|l| l == "Person"));
    measure!(
        measurements,
        "get_popular_labels",
        storage.get_popular_labels(5, Some(&tid), Some(&wid))
    );

    let popular_nodes = storage
        .get_popular_nodes_with_degree(5, Some(1), Some("Person"), Some(&tid), Some(&wid))
        .await
        .expect("popular nodes");
    assert!(popular_nodes.iter().any(|(n, d)| n.id == "BOB" && *d >= 1));
    measure!(
        measurements,
        "get_popular_nodes_with_degree",
        storage.get_popular_nodes_with_degree(5, Some(1), Some("Person"), Some(&tid), Some(&wid))
    );

    let page = storage
        .list_nodes_filtered(&node_filter, 0, 10)
        .await
        .expect("list nodes");
    assert_eq!(page.total, 3);
    measure!(
        measurements,
        "list_nodes_filtered",
        storage.list_nodes_filtered(&node_filter, 0, 10)
    );

    let epage = storage
        .list_edges_filtered(&edge_filter, 0, 10)
        .await
        .expect("list edges");
    assert_eq!(epage.total, 2);
    measure!(
        measurements,
        "list_edges_filtered",
        storage.list_edges_filtered(&edge_filter, 0, 10)
    );

    let keyset = storage
        .scan_nodes_after(&node_filter, None, 2)
        .await
        .expect("scan nodes");
    assert_eq!(keyset.items.len(), 2);
    assert!(keyset.next_after.is_some());
    measure!(
        measurements,
        "scan_nodes_after",
        storage.scan_nodes_after(&node_filter, None, 2)
    );

    let ekey = storage
        .scan_edges_after(&edge_filter, None, 1)
        .await
        .expect("scan edges");
    assert_eq!(ekey.items.len(), 1);
    let edge_cursor = ekey.next_after.clone().expect("edge cursor");
    let ekey2 = storage
        .scan_edges_after(&edge_filter, Some(&edge_cursor), 1)
        .await
        .expect("scan edges page 2");
    assert_eq!(ekey2.items.len(), 1);
    assert_ne!(ekey.items[0].source, ekey2.items[0].source);

    let typed = NodeListFilter {
        entity_type: Some("person".into()),
        ..node_filter.clone()
    };
    let people = storage
        .list_nodes_filtered(&typed, 0, 10)
        .await
        .expect("type page");
    let ids: Vec<_> = people.items.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"ALICE") && ids.contains(&"BOB"));
    assert!(!ids.contains(&"CAROL"));
    measure!(
        measurements,
        "scan_edges_after",
        storage.scan_edges_after(&edge_filter, None, 10)
    );

    let by_src = storage
        .find_nodes_by_source_prefixes(&node_filter, &["doc1-chunk-".into()])
        .await
        .expect("lineage nodes");
    assert!(by_src.iter().any(|n| n.id == "ALICE"));
    measure!(
        measurements,
        "find_nodes_by_source_prefixes",
        storage.find_nodes_by_source_prefixes(&node_filter, &["doc1-chunk-".into()])
    );

    let by_src_e = storage
        .find_edges_by_source_prefixes(&edge_filter, &["doc1-chunk-".into()])
        .await
        .expect("lineage edges");
    assert!(!by_src_e.is_empty());
    measure!(
        measurements,
        "find_edges_by_source_prefixes",
        storage.find_edges_by_source_prefixes(&edge_filter, &["doc1-chunk-".into()])
    );

    let by_rel = storage
        .find_edge_by_relationship_id(&edge_filter, "rel-alice-bob")
        .await
        .expect("rel id")
        .expect("found rel");
    assert_eq!(by_rel.source, "ALICE");
    measure!(
        measurements,
        "find_edge_by_relationship_id",
        storage.find_edge_by_relationship_id(&edge_filter, "rel-alice-bob")
    );

    assert_eq!(storage.node_count().await.expect("nc"), 4);
    assert_eq!(storage.edge_count().await.expect("ec"), 2);
    assert_eq!(
        storage.node_count_by_workspace(&ws_a).await.expect("ncw"),
        3
    );
    assert_eq!(
        storage.edge_count_by_workspace(&ws_a).await.expect("ecw"),
        2
    );
    assert!(
        storage
            .distinct_node_type_count_by_workspace(&ws_a)
            .await
            .expect("types")
            >= 2
    );
    let prefix_counts = storage
        .node_counts_by_source_prefixes(&["doc1-chunk-".into()])
        .await
        .expect("prefix counts");
    assert!(prefix_counts.get("doc1-chunk-").copied().unwrap_or(0) >= 1);
    storage.ping().await.expect("ping");
    measure!(measurements, "node_count", storage.node_count());
    measure!(measurements, "edge_count", storage.edge_count());
    measure!(
        measurements,
        "node_count_by_workspace",
        storage.node_count_by_workspace(&ws_a)
    );
    measure!(
        measurements,
        "node_counts_by_source_prefixes",
        storage.node_counts_by_source_prefixes(&["doc1-chunk-".into()])
    );
    measure!(measurements, "ping", storage.ping());

    storage
        .upsert_nodes_batch_with_mode(
            &[node(
                "ALICE",
                &tenant_a,
                &ws_a,
                "Person",
                "alice example",
                &["doc1-chunk-9"],
            )],
            GraphPropertyWriteMode::MergeSources,
        )
        .await
        .expect("merge");
    let merged = storage
        .get_node("ALICE")
        .await
        .expect("merged")
        .expect("alice");
    let sources = merged
        .properties
        .get("source_ids")
        .cloned()
        .unwrap_or(Value::Null);
    let text = sources.to_string();
    assert!(
        text.contains("doc1-chunk-0") && text.contains("doc1-chunk-9"),
        "{text}"
    );

    storage
        .upsert_nodes_batch_with_mode(
            &[node(
                "ALICE",
                &tenant_a,
                &ws_a,
                "Person",
                "replaced",
                &["only-new"],
            )],
            GraphPropertyWriteMode::Replace,
        )
        .await
        .expect("replace");
    let replaced = storage
        .get_node("ALICE")
        .await
        .expect("replaced")
        .expect("alice");
    let sources = replaced
        .properties
        .get("source_ids")
        .cloned()
        .unwrap_or(Value::Null);
    assert!(sources.to_string().contains("only-new"));
    assert!(!sources.to_string().contains("doc1-chunk-0"));
    assert_eq!(
        storage
            .node_degree("ALICE")
            .await
            .expect("degree after replace"),
        1,
        "replacing node properties must not reset degree"
    );

    prove_rls(&pool, &tenant_a, &tenant_b).await;

    storage
        .delete_edge("BOB", "CAROL")
        .await
        .expect("delete edge");
    assert!(storage
        .get_edge("BOB", "CAROL")
        .await
        .expect("gone")
        .is_none());
    assert_eq!(
        storage.node_degree("BOB").await.expect("bob after delete"),
        1
    );
    storage.delete_node("CAROL").await.expect("delete node");
    assert!(!storage.has_node("CAROL").await.expect("has carol"));
    let (n, _e) = storage.clear_workspace(&ws_b).await.expect("clear ws");
    assert_eq!(n, 1);
    assert!(!storage.has_node("DAVE").await.expect("dave"));

    sqlx::query("UPDATE public.eq_graph_cutover SET status = 'age', shadow_ok = false, backfill_complete = false WHERE id = 1")
        .execute(&pool)
        .await
        .expect("red ledger");
    let red = sqlx::raw_sql(M171).execute(&pool).await;
    assert!(red.is_err(), "171 must refuse a red ledger");
    sqlx::query("UPDATE public.eq_graph_cutover SET status = 'ready', shadow_ok = true, backfill_complete = true, age_absent = true WHERE id = 1")
        .execute(&pool)
        .await
        .expect("green ledger");
    sqlx::raw_sql(M171)
        .execute(&pool)
        .await
        .expect("171 greenfield");
    assert!(storage.has_node("ALICE").await.expect("still there"));

    let body = serde_json::to_string_pretty(&Value::Object(measurements)).unwrap();
    std::fs::write("/tmp/spec164-measurements.json", &body).expect("write measurements");
    eprintln!("{body}");

    storage.clear().await.expect("clear");
    assert_eq!(storage.node_count().await.expect("empty"), 0);

    pool.close().await;
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin)
        .await
        .expect("admin reconnect");
    sqlx::query(&format!("DROP DATABASE {db}"))
        .execute(&admin_pool)
        .await
        .ok();
}

fn node(
    id: &str,
    tenant: &Uuid,
    workspace: &Uuid,
    entity_type: &str,
    description: &str,
    sources: &[&str],
) -> (String, HashMap<String, Value>) {
    let mut properties = HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("entity_type".into(), json!(entity_type));
    properties.insert("description".into(), json!(description));
    properties.insert("source_ids".into(), json!(sources));
    (id.to_string(), properties)
}

fn edge(
    source: &str,
    target: &str,
    rel: &str,
    tenant: &Uuid,
    workspace: &Uuid,
    relationship_id: &str,
) -> (String, String, HashMap<String, Value>) {
    let mut properties = HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("relation_type".into(), json!(rel));
    properties.insert("id".into(), json!(relationship_id));
    properties.insert("source_ids".into(), json!(["doc1-chunk-0"]));
    (source.into(), target.into(), properties)
}

async fn prove_rls(pool: &sqlx::PgPool, tenant_a: &Uuid, tenant_b: &Uuid) {
    let mut conn = pool.acquire().await.expect("rls conn");
    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_a.to_string())
        .execute(&mut *conn)
        .await
        .expect("set tenant");
    sqlx::query("SET ROLE edgequake_tenant_access")
        .execute(&mut *conn)
        .await
        .expect("set role");
    let visible: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM public.eq_graph_nodes WHERE namespace = 'spec164'",
    )
    .fetch_one(&mut *conn)
    .await
    .expect("rls count");
    assert!(
        (1..4).contains(&visible),
        "tenant A must not see every node, saw {visible}"
    );
    let leaked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM public.eq_graph_nodes WHERE namespace = 'spec164' AND tenant_id = $1",
    )
    .bind(tenant_b.to_string())
    .fetch_one(&mut *conn)
    .await
    .expect("leak count");
    assert_eq!(leaked, 0, "tenant A must not read tenant B rows");
    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");
}

fn admin_url() -> Option<String> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::fs::read_to_string("/tmp/edgequake-db-url").ok())?;
    let url = url.trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn replace_db(url: &str, db: &str) -> String {
    let q = url.find('?').map(|i| &url[i..]).unwrap_or("");
    let without_q = url.trim_end_matches(q);
    let slash = without_q.rfind('/').expect("db slash");
    format!("{}{}{}", &without_q[..slash + 1], db, q)
}

fn parse_url(url: &str) -> (String, u16, String, String) {
    let without_scheme = url.split("://").nth(1).expect("scheme");
    let (auth, host_path) = without_scheme.split_once('@').expect("auth");
    let (user, password) = auth.split_once(':').expect("user");
    let (host_port, _db) = host_path.split_once('/').expect("host");
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(5432)),
        None => (host_port.to_string(), 5432),
    };
    (host, port, user.to_string(), password.to_string())
}

const STUBS: &str = r#"
DO $$ BEGIN
    CREATE ROLE edgequake_tenant_access NOLOGIN;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
-- Only the columns the scope check reads; the real table is migration 001.
CREATE TABLE IF NOT EXISTS public.workspaces (
    workspace_id uuid PRIMARY KEY,
    tenant_id    uuid NOT NULL,
    is_active    boolean NOT NULL DEFAULT true
);
CREATE OR REPLACE FUNCTION public.current_tenant_id() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT NULLIF(current_setting('app.current_tenant_id', true), '')::uuid
$$;
CREATE OR REPLACE FUNCTION public.current_workspace_id() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT NULLIF(current_setting('app.current_workspace_id', true), '')::uuid
$$;
"#;
