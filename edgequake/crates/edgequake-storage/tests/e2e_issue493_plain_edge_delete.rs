//! Issue #493: once the plain-graph ledger is ready, document delete must remove
//! `eq_graph_edges` rows (and prune shared provenance) instead of writing only
//! to the frozen Apache AGE archive.

use std::collections::HashMap;

use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps};
use edgequake_storage::{GraphPropertyWriteMode, PostgresAGEGraphStorage, PostgresConfig};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use uuid::Uuid;

const M090: &str = include_str!("../../../migrations/090_eq_merge_graph_properties.sql");
const M170: &str = include_str!("../../../migrations/170_spec164_plain_graph.sql");

#[tokio::test]
#[allow(deprecated)]
async fn ready_ledger_edge_delete_updates_plain_provenance() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP issue493 e2e: set DATABASE_URL to a Postgres that can CREATE DATABASE");
        return;
    };
    let db = format!("issue493_{}", &Uuid::new_v4().simple().to_string()[..12]);
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
    // Template databases often omit AGE. The cluster still has the extension
    // files when the dev database uses it. Creating it before initialize lets
    // the red-ledger section build a real graph; the cutover row stays `ready`
    // because migration 170 already ran with zero graphs.
    if sqlx::raw_sql("CREATE EXTENSION IF NOT EXISTS age")
        .execute(&pool)
        .await
        .is_err()
    {
        eprintln!("issue493: Apache AGE extension is not available on this server");
    }

    let (host, port, user, password) = parse_url(&url);
    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();
    let config = PostgresConfig::new(host, port, &db, user, password)
        .with_namespace("issue493")
        .with_max_connections(4);
    let storage = PostgresAGEGraphStorage::new(config.clone());
    storage.initialize().await.expect("initialize plain graph");

    let status: String =
        sqlx::query_scalar("SELECT status FROM public.eq_graph_cutover WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("ledger");
    assert_eq!(status, "ready", "greenfield ledger must be ready");

    let old_chunk = "old-doc-chunk-0";
    let other_chunk = "other-doc-chunk-0";
    let new_chunk = "new-doc-chunk-0";
    storage
        .upsert_nodes_batch(&[
            node("N1", &tenant_a, &ws_a),
            node("N2", &tenant_a, &ws_a),
            node("N3", &tenant_b, &ws_b),
            node("N4", &tenant_b, &ws_b),
            node("N5", &tenant_a, &ws_a),
        ])
        .await
        .expect("nodes");
    storage
        .upsert_edges_batch(&[
            edge("N1", "N2", "KNOWS", &tenant_a, &ws_a, &[old_chunk]),
            edge("N1", "N2", "WORKS_WITH", &tenant_a, &ws_a, &[other_chunk]),
            edge(
                "N2",
                "N1",
                "CITES",
                &tenant_a,
                &ws_a,
                &[old_chunk, other_chunk],
            ),
            edge("N3", "N4", "KNOWS", &tenant_b, &ws_b, &[old_chunk]),
            edge("N5", "N2", "KNOWS", &tenant_a, &ws_a, &[old_chunk]),
        ])
        .await
        .expect("edges");

    assert_eq!(storage.node_degree("N1").await.expect("degree"), 3);

    storage
        .delete_edges_batch(&[("N1".into(), "N2".into(), "KNOWS".into())])
        .await
        .expect("delete exclusive sister");
    assert!(
        edge_row(&pool, "N1", "N2", "KNOWS").await.is_none(),
        "exclusive plain edge must be removed"
    );
    assert!(
        storage.has_node("N1").await.expect("n1") && storage.has_node("N2").await.expect("n2"),
        "shared endpoints stay"
    );
    let sister = edge_row(&pool, "N1", "N2", "WORKS_WITH")
        .await
        .expect("sister rel");
    assert!(
        cites(&sister, other_chunk) && !cites(&sister, old_chunk),
        "sister provenance: {sister}"
    );
    assert_eq!(
        storage
            .node_degree("N1")
            .await
            .expect("degree after delete"),
        2,
        "degree trigger must see the plain delete"
    );

    storage
        .upsert_edges_batch(&[edge("N1", "N2", "KNOWS", &tenant_a, &ws_a, &[new_chunk])])
        .await
        .expect("re-ingest merge");
    let reborn = edge_row(&pool, "N1", "N2", "KNOWS")
        .await
        .expect("re-ingested edge");
    assert!(
        cites(&reborn, new_chunk) && !cites(&reborn, old_chunk),
        "merge onto a deleted edge must not resurrect the old document: {reborn}"
    );

    storage
        .upsert_edges_batch_with_mode(
            &[edge("N2", "N1", "CITES", &tenant_a, &ws_a, &[other_chunk])],
            GraphPropertyWriteMode::Replace,
        )
        .await
        .expect("shared replace");
    let shared = edge_row(&pool, "N2", "N1", "CITES")
        .await
        .expect("shared edge");
    assert!(
        cites(&shared, other_chunk) && !cites(&shared, old_chunk),
        "shared edge must drop the deleted document: {shared}"
    );

    let removed = storage
        .delete_edges_scoped_batch(
            &[("N1".into(), "N2".into(), "knows".into())],
            &tenant_a.to_string(),
            &ws_a.to_string(),
        )
        .await
        .expect("scoped sister delete");
    assert_eq!(
        removed, 1,
        "scoped delete removes the one rel, case-insensitively"
    );
    assert!(edge_row(&pool, "N1", "N2", "KNOWS").await.is_none());
    assert!(
        edge_row(&pool, "N1", "N2", "WORKS_WITH").await.is_some(),
        "scoped delete must not remove the other relationship"
    );
    let untouched = storage
        .delete_edges_scoped_batch(
            &[("N3".into(), "N4".into(), "KNOWS".into())],
            &tenant_a.to_string(),
            &ws_a.to_string(),
        )
        .await
        .expect("cross-tenant delete");
    assert_eq!(untouched, 0);
    assert!(
        edge_row(&pool, "N3", "N4", "KNOWS").await.is_some(),
        "another tenant's edge stays"
    );

    let nodes_removed = storage
        .delete_nodes_scoped_batch(&["N5".into()], &tenant_a.to_string(), &ws_a.to_string())
        .await
        .expect("scoped node delete");
    assert_eq!(nodes_removed, 1);
    assert!(!storage.has_node("N5").await.expect("n5"));
    assert!(
        edge_row(&pool, "N5", "N2", "KNOWS").await.is_none(),
        "incident plain edge is detached"
    );
    assert!(storage.has_node("N2").await.expect("n2 stays"));
    let wrong_tenant = storage
        .delete_nodes_scoped_batch(&["N2".into()], &tenant_b.to_string(), &ws_b.to_string())
        .await
        .expect("node fence");
    assert_eq!(wrong_tenant, 0);
    assert!(storage.has_node("N2").await.expect("n2 fenced"));

    // Backfill can store the raw label when `eq_rel_type` was empty. The
    // delete arbiter has to uppercase both the column and the bound label.
    sqlx::query(
        "INSERT INTO public.eq_graph_edges
            (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, properties)
         VALUES ('issue493', 'N1', 'N2', 'mentions', $1, $2, $3::jsonb)",
    )
    .bind(tenant_a.to_string())
    .bind(ws_a.to_string())
    .bind(json!({"relation_type": "mentions", "source_ids": [old_chunk], "source_chunk_ids": [old_chunk]}))
    .execute(&pool)
    .await
    .expect("raw rel insert");
    storage
        .delete_edges_batch(&[("N1".into(), "N2".into(), "Mentions".into())])
        .await
        .expect("delete raw rel");
    let raw_left: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.eq_graph_edges
         WHERE namespace = 'issue493' AND source_id = 'N1' AND target_id = 'N2'
           AND rel_type = 'mentions'",
    )
    .fetch_one(&pool)
    .await
    .expect("raw rel count");
    assert_eq!(
        raw_left, 0,
        "raw stored rel_type must match the SQL arbiter"
    );

    prove_age_ledger_delete(&pool, config).await;

    storage.clear().await.expect("clear");
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

async fn prove_age_ledger_delete(pool: &sqlx::PgPool, config: PostgresConfig) {
    let age: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(pool)
            .await
            .expect("age probe");
    if !age {
        eprintln!("SKIP issue493 age-ledger delete: Apache AGE is not installed");
        return;
    }
    // A pool opened while the ledger is `ready` keeps AGE disabled. A new pool
    // opened after the ledger is `age` is what exercises the mirror path.
    sqlx::query(
        "UPDATE public.eq_graph_cutover
         SET status = 'age', age_absent = false, shadow_ok = false, backfill_complete = false
         WHERE id = 1",
    )
    .execute(pool)
    .await
    .expect("red ledger");
    // Graph creation calls this installer. The full tenant-RLS migration needs
    // the product schemas; a no-op is enough to prove the AGE delete mirror.
    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION public.install_graph_tenant_rls(graph_name text)
         RETURNS void LANGUAGE plpgsql AS $$ BEGIN RETURN; END $$",
    )
    .execute(pool)
    .await
    .expect("rls stub");
    let storage = PostgresAGEGraphStorage::new(config);
    storage.initialize().await.expect("initialize AGE graph");
    sqlx::query("SELECT public.eq_graph_install_mirror_triggers($1, $2)")
        .bind("eq_eq_issue493_graph")
        .bind("issue493")
        .execute(pool)
        .await
        .expect("mirror triggers");
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let tid = tenant.to_string();
    let wid = workspace.to_string();
    // Cypher creates the AGE row (and the mirror copies it). Native upsert needs
    // the full graph-id helper chain, which this fixture does not build.
    let create = format!(
        "SELECT * FROM cypher('eq_eq_issue493_graph', $$
            CREATE (:Node {{node_id: 'AGE_A', tenant_id: '{tid}', workspace_id: '{wid}'}})
            -[:EDGE {{source_id: 'AGE_A', target_id: 'AGE_B', relation_type: 'KNOWS', tenant_id: '{tid}', workspace_id: '{wid}'}}]->
            (:Node {{node_id: 'AGE_B', tenant_id: '{tid}', workspace_id: '{wid}'}})
         $$) AS (v agtype)"
    );
    let mut conn = pool.acquire().await.expect("age conn");
    if let Err(err) = async {
        sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
        sqlx::query("SET search_path = ag_catalog, public")
            .execute(&mut *conn)
            .await?;
        sqlx::query(&create).execute(&mut *conn).await?;
        Ok::<(), sqlx::Error>(())
    }
    .await
    {
        eprintln!("SKIP issue493 age-ledger delete: could not create an AGE edge ({err})");
        return;
    }
    // Cypher CREATE does not fire the plain-graph mirror. A SQL update does,
    // which is the same trigger `delete_edges_batch` relies on.
    sqlx::query(r#"UPDATE eq_eq_issue493_graph."EDGE" SET properties = properties"#)
        .execute(&mut *conn)
        .await
        .expect("touch AGE edge so the mirror runs");
    drop(conn);
    assert!(
        edge_row(pool, "AGE_A", "AGE_B", "KNOWS").await.is_some(),
        "AGE insert must mirror into eq_graph_edges"
    );
    storage
        .delete_edges_batch(&[("AGE_A".into(), "AGE_B".into(), "KNOWS".into())])
        .await
        .expect("age batch delete");
    assert!(storage
        .get_edge("AGE_A", "AGE_B")
        .await
        .expect("read")
        .is_none());
    assert!(
        edge_row(pool, "AGE_A", "AGE_B", "KNOWS").await.is_none(),
        "AGE delete must mirror away the plain row"
    );
}

fn node(id: &str, tenant: &Uuid, workspace: &Uuid) -> (String, HashMap<String, Value>) {
    let mut properties = HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("entity_type".into(), json!("Person"));
    properties.insert("description".into(), json!(id));
    (id.to_string(), properties)
}

fn edge(
    source: &str,
    target: &str,
    rel: &str,
    tenant: &Uuid,
    workspace: &Uuid,
    chunks: &[&str],
) -> (String, String, HashMap<String, Value>) {
    let mut properties = HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("relation_type".into(), json!(rel));
    properties.insert("source_ids".into(), json!(chunks));
    properties.insert("source_chunk_ids".into(), json!(chunks));
    (source.into(), target.into(), properties)
}

fn cites(properties: &Value, chunk: &str) -> bool {
    let hit = |key: &str| {
        properties
            .get(key)
            .and_then(|v| v.as_array())
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(chunk)))
    };
    hit("source_ids") || hit("source_chunk_ids")
}

async fn edge_row(pool: &sqlx::PgPool, source: &str, target: &str, rel: &str) -> Option<Value> {
    sqlx::query(
        "SELECT properties FROM public.eq_graph_edges
         WHERE namespace = 'issue493' AND source_id = $1 AND target_id = $2 AND rel_type = $3",
    )
    .bind(source)
    .bind(target)
    .bind(rel)
    .fetch_optional(pool)
    .await
    .expect("edge row")
    .map(|row| row.get("properties"))
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
CREATE OR REPLACE FUNCTION public.current_tenant_id() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT NULLIF(current_setting('app.current_tenant_id', true), '')::uuid
$$;
CREATE OR REPLACE FUNCTION public.current_workspace_id() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT NULLIF(current_setting('app.current_workspace_id', true), '')::uuid
$$;
"#;
