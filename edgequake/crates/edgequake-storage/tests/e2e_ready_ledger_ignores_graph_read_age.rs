//! A ready cutover ignores `EDGEQUAKE_GRAPH_READ=age`.
//!
//! The scratch database has Apache AGE installed and `eq_graph_cutover.status = ready`
//! (migration 170 greenfield: extension present, no graphs). Initializing storage
//! with the variable set must not create AGE label tables. Reads and writes use
//! `eq_graph_nodes` / `eq_graph_edges`. EXPLAIN for that path must not mention
//! `ag_catalog` or AGE label tables.
//!
//! Run:
//!   cargo test -p edgequake-storage --features postgres \
//!     --test e2e_ready_ledger_ignores_graph_read_age -- --test-threads=1

#![cfg(feature = "postgres")]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use edgequake_storage::{
    GraphStorage, GraphStorageMutateOps, GraphStorageReadOps, PostgresAGEGraphStorage,
    PostgresConfig,
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use uuid::Uuid;

const M090: &str = include_str!("../../../migrations/090_eq_merge_graph_properties.sql");
const M170: &str = include_str!("../../../migrations/170_spec164_plain_graph.sql");

#[tokio::test]
async fn ready_ledger_ignores_graph_read_age() {
    let Some(admin_url) = admin_url() else {
        eprintln!("SKIP: no DATABASE_URL / /tmp/edgequake-db-url");
        return;
    };
    let db = format!("ready_age_{}", &Uuid::new_v4().simple().to_string()[..12]);
    assert!(
        db.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
        "unsafe database name {db}"
    );
    let _cleanup = DropDatabase {
        admin_url: admin_url.clone(),
        name: db.clone(),
    };

    std::env::set_var("EDGEQUAKE_GRAPH_READ", "age");
    let capture = NoticeCapture::new();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(capture.clone()))
        .expect("tracing subscriber");

    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&rewrite_database(&admin_url, "postgres"))
        .await
        .expect("admin connect");
    sqlx::query(&format!("CREATE DATABASE {db}"))
        .execute(&admin)
        .await
        .expect("create database");
    admin.close().await;

    let url = rewrite_database(&admin_url, &db);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("scratch connect");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE")
        .execute(&pool)
        .await
        .expect("Apache AGE must be installed so a wrong pin could create label tables");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&pool)
        .await
        .expect("pgvector");
    sqlx::raw_sql("SET search_path TO public")
        .execute(&pool)
        .await
        .expect("search_path");
    sqlx::raw_sql(M090)
        .execute(&pool)
        .await
        .expect("migration 090");
    sqlx::raw_sql(M170)
        .execute(&pool)
        .await
        .expect("migration 170");

    let (status, age_absent): (String, bool) =
        sqlx::query_as("SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("ledger");
    assert_eq!(status, "ready", "170 greenfield ledger must be ready");
    assert!(
        !age_absent,
        "AGE is installed; age_absent must stay false so the pin could reopen it"
    );
    let graphs_before: i64 = sqlx::query_scalar("SELECT count(*)::bigint FROM ag_catalog.ag_graph")
        .fetch_one(&pool)
        .await
        .expect("ag_graph before");
    assert_eq!(
        graphs_before, 0,
        "scratch database starts with no AGE graphs"
    );

    enable_auto_explain(&db, &admin_url).await;
    pool.close().await;

    let (host, port, user, password) = parse_url(&url);
    let config = PostgresConfig::new(host, port, &db, user, password)
        .with_namespace("readyage")
        .with_max_connections(4);
    let graph_name = config.age_graph_name();
    let storage = PostgresAGEGraphStorage::new(config);
    storage
        .initialize()
        .await
        .expect("initialize on a ready ledger with EDGEQUAKE_GRAPH_READ=age");

    let check = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("check connect");
    assert_no_age_labels(&check, &graph_name).await;

    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    capture.arm();
    storage
        .upsert_nodes_batch(&[
            node("ALICE", &tenant, &workspace),
            node("BOB", &tenant, &workspace),
        ])
        .await
        .expect("upsert nodes");
    storage
        .upsert_edges_batch(&[edge("ALICE", "BOB", &tenant, &workspace)])
        .await
        .expect("upsert edges");
    let alice = storage
        .get_node("ALICE")
        .await
        .expect("get node")
        .expect("ALICE was written to the plain tables");
    assert_eq!(alice.id, "ALICE");
    let knows = storage
        .get_edge("ALICE", "BOB")
        .await
        .expect("get edge")
        .expect("edge was written to the plain tables");
    assert_eq!(knows.source, "ALICE");
    assert_eq!(knows.target, "BOB");
    assert_eq!(
        storage.node_degree("ALICE").await.expect("degree"),
        1,
        "degree is maintained on eq_graph_nodes"
    );
    capture.disarm();

    let nodes: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.eq_graph_nodes WHERE namespace = 'readyage'",
    )
    .fetch_one(&check)
    .await
    .expect("plain node count");
    let edges: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.eq_graph_edges WHERE namespace = 'readyage'",
    )
    .fetch_one(&check)
    .await
    .expect("plain edge count");
    assert_eq!(nodes, 2, "both nodes live in eq_graph_nodes");
    assert_eq!(edges, 1, "the edge lives in eq_graph_edges");
    assert_no_age_labels(&check, &graph_name).await;

    let plans = capture.lines();
    assert!(
        !plans.is_empty(),
        "auto_explain produced no plans for the plain read/write path"
    );
    let joined = plans.join("\n");
    assert!(
        joined.contains("eq_graph_nodes"),
        "EXPLAIN for the path must read or write eq_graph_nodes:\n{joined}"
    );
    assert!(
        joined.contains("eq_graph_edges"),
        "EXPLAIN for the path must read or write eq_graph_edges:\n{joined}"
    );
    assert_plain_plan(&joined);
    eprintln!("READY_LEDGER_EXPLAIN\n{joined}");
    check.close().await;
}

fn assert_plain_plan(plan: &str) {
    let folded = plan.to_ascii_lowercase();
    assert!(
        !folded.contains("ag_catalog"),
        "EXPLAIN mentions ag_catalog:\n{plan}"
    );
    for needle in ["\"node\"", "\"edge\"", "_ag_label_vertex", "_ag_label_edge"] {
        assert!(
            !folded.contains(needle),
            "EXPLAIN mentions an AGE label table ({needle}):\n{plan}"
        );
    }
}

async fn assert_no_age_labels(pool: &PgPool, graph_name: &str) {
    let graphs: i64 = sqlx::query_scalar("SELECT count(*)::bigint FROM ag_catalog.ag_graph")
        .fetch_one(pool)
        .await
        .expect("ag_graph");
    assert_eq!(graphs, 0, "EDGEQUAKE_GRAPH_READ=age created an AGE graph");
    let schema_exists: bool = sqlx::query_scalar("SELECT to_regnamespace($1) IS NOT NULL")
        .bind(graph_name)
        .fetch_one(pool)
        .await
        .expect("graph schema");
    assert!(
        !schema_exists,
        "AGE graph schema {graph_name} exists after a ready ledger"
    );
    let labels: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint
         FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE c.relkind = 'r'
           AND c.relname IN ('Node', 'EDGE', '_ag_label_vertex', '_ag_label_edge')
           AND n.nspname NOT IN ('pg_catalog', 'information_schema')",
    )
    .fetch_one(pool)
    .await
    .expect("label tables");
    assert_eq!(
        labels, 0,
        "ready ledger plus EDGEQUAKE_GRAPH_READ=age created AGE label tables"
    );
}

async fn enable_auto_explain(db: &str, admin_url: &str) {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&rewrite_database(admin_url, db))
        .await
        .expect("auto_explain connect");
    for stmt in [
        format!("ALTER DATABASE {db} SET session_preload_libraries = 'auto_explain'"),
        format!("ALTER DATABASE {db} SET auto_explain.log_min_duration = '0'"),
        format!("ALTER DATABASE {db} SET auto_explain.log_analyze = 'false'"),
        format!("ALTER DATABASE {db} SET auto_explain.log_nested_statements = 'on'"),
        format!("ALTER DATABASE {db} SET auto_explain.log_level = 'notice'"),
    ] {
        sqlx::query(&stmt)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("{stmt} failed: {e}"));
    }
    pool.close().await;
}

fn node(id: &str, tenant: &Uuid, workspace: &Uuid) -> (String, HashMap<String, Value>) {
    let mut properties = HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("entity_type".into(), json!("Person"));
    properties.insert("description".into(), json!(id));
    properties.insert("source_ids".into(), json!(["doc-chunk-0"]));
    (id.to_string(), properties)
}

fn edge(
    source: &str,
    target: &str,
    tenant: &Uuid,
    workspace: &Uuid,
) -> (String, String, HashMap<String, Value>) {
    let mut properties = HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("relation_type".into(), json!("KNOWS"));
    properties.insert("id".into(), json!("rel-alice-bob"));
    properties.insert("source_ids".into(), json!(["doc-chunk-0"]));
    (source.into(), target.into(), properties)
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

fn rewrite_database(url: &str, name: &str) -> String {
    let (base, rest) = url.rsplit_once('/').expect("database url");
    match rest.split_once('?') {
        Some((_, query)) => format!("{base}/{name}?{query}"),
        None => format!("{base}/{name}"),
    }
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

#[derive(Clone)]
struct NoticeCapture {
    armed: Arc<AtomicBool>,
    lines: Arc<Mutex<Vec<String>>>,
}

impl NoticeCapture {
    fn new() -> Self {
        Self {
            armed: Arc::new(AtomicBool::new(false)),
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn arm(&self) {
        self.lines.lock().expect("plans").clear();
        self.armed.store(true, Ordering::Relaxed);
    }

    fn disarm(&self) {
        self.armed.store(false, Ordering::Relaxed);
    }

    fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("plans").clone()
    }
}

struct MessageVisitor<'a>(&'a mut Option<String>);

impl Visit for MessageVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            *self.0 = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && self.0.is_none() {
            *self.0 = Some(format!("{value:?}"));
        }
    }
}

impl<S> Layer<S> for NoticeCapture
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != "sqlx::postgres::notice" {
            return;
        }
        if !self.armed.load(Ordering::Relaxed) {
            return;
        }
        let mut message = None;
        event.record(&mut MessageVisitor(&mut message));
        if let Some(message) = message {
            self.lines.lock().expect("plans").push(message);
        }
    }
}

struct DropDatabase {
    admin_url: String,
    name: String,
}

impl Drop for DropDatabase {
    fn drop(&mut self) {
        let admin_url = self.admin_url.clone();
        let name = self.name.clone();
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("drop runtime");
            rt.block_on(async move {
                let Ok(admin) = PgPoolOptions::new()
                    .max_connections(1)
                    .connect(&rewrite_database(&admin_url, "postgres"))
                    .await
                else {
                    return;
                };
                let _ = sqlx::query(
                    "SELECT pg_terminate_backend(pid)
                     FROM pg_stat_activity
                     WHERE datname = $1 AND pid <> pg_backend_pid()",
                )
                .bind(&name)
                .execute(&admin)
                .await;
                let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {name}"))
                    .execute(&admin)
                    .await;
                admin.close().await;
            });
        })
        .join();
    }
}
