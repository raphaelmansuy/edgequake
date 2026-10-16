//! GitHub #494: a role or database `statement_timeout` must not cancel the
//! plain-graph cutover, and a cap that does fire must roll the copy back.
//!
//! The large graph matches the reported failure: 166,067 nodes and 268,144 edges.
//! Skipped unless `DATABASE_URL` points at a server that can `CREATE EXTENSION age`.

use std::time::{Duration, Instant};

use edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations;
use edgequake_storage::{
    finish_plain_graph_cutover, GraphStorage, PostgresAGEGraphStorage, PostgresConfig,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Reported production shape (issue #494).
const LARGE_NODES: i64 = 166_067;
const LARGE_EDGES: i64 = 268_144;
/// A few thousand rows, fat enough that a 50ms cap cancels the copy.
const ROLLBACK_NODES: i64 = 8_000;
const ROLLBACK_EDGES: i64 = 12_000;

#[tokio::test]
async fn issue494_cutover_ignores_database_timeout_and_cap_rolls_back() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP issue494: set DATABASE_URL");
        return;
    };
    if std::env::var("EDGEQUAKE_SPEC164_NO_AGE").is_ok() {
        eprintln!("SKIP issue494: EDGEQUAKE_SPEC164_NO_AGE is set");
        return;
    }
    std::env::remove_var("EDGEQUAKE_CUTOVER_STATEMENT_TIMEOUT");
    let large = format!("i494l_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let small = format!("i494s_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let result = run(&admin, &large, &small).await;
    drop_db(&admin, &large).await;
    drop_db(&admin, &small).await;
    if let Err(err) = result {
        panic!("{err}");
    }
}

async fn run(admin: &str, large_db: &str, small_db: &str) -> Result<(), String> {
    prove_large_graph(admin, large_db).await?;
    prove_cap_rolls_back(admin, small_db).await?;
    Ok(())
}

async fn prove_large_graph(admin: &str, db: &str) -> Result<(), String> {
    let url = provision(admin, db).await?;
    let open = connect(&url).await?;
    if !prepare_graph(&open).await? {
        eprintln!("SKIP issue494 large graph: Apache AGE is not available");
        return Ok(());
    }
    let graph = graph_name(&url, "issue494").await?;
    let (tenant, workspace) = insert_scope(&open).await?;
    load_ring(&open, &graph, tenant, workspace, LARGE_NODES, LARGE_EDGES).await?;
    open.close().await;

    alter_database_timeout(admin, db, "1s").await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .map_err(|e| e.to_string())?;
    assert_statement_cancelled(&pool, "database statement_timeout=1s before cutover").await?;
    let shown = show_statement_timeout(&pool).await?;
    if shown != "1s" && shown != "1000ms" {
        return Err(format!(
            "expected database statement_timeout 1s, SHOW returned {shown}"
        ));
    }

    let started = Instant::now();
    let report = finish_plain_graph_cutover(&pool)
        .await
        .map_err(|e| format!("large cutover failed under a 1s database timeout: {e}"))?;
    let elapsed = started.elapsed();
    eprintln!(
        "issue494 large cutover: ready={} graphs={} mismatches={} elapsed={elapsed:?} note={}",
        report.ready, report.graphs, report.mismatches, report.note
    );
    if !report.ready || report.mismatches != 0 {
        return Err(format!("large cutover did not become ready: {report:?}"));
    }
    if elapsed < Duration::from_secs(1) {
        return Err(format!(
            "copy finished in {elapsed:?}, under the 1s database timeout, so this run does not prove the override"
        ));
    }
    let plain_nodes = count_plain(&pool, "issue494", "eq_graph_nodes").await?;
    let plain_edges = count_plain(&pool, "issue494", "eq_graph_edges").await?;
    if plain_nodes != LARGE_NODES || plain_edges != LARGE_EDGES {
        return Err(format!(
            "plain counts nodes={plain_nodes} edges={plain_edges}, AGE graph was {LARGE_NODES}/{LARGE_EDGES}"
        ));
    }
    let status = ledger_status(&pool).await?;
    if status.as_deref() != Some("ready") {
        return Err(format!("ledger status after success: {status:?}"));
    }
    let shown_after = show_statement_timeout(&pool).await?;
    if shown_after != "1s" && shown_after != "1000ms" {
        return Err(format!(
            "SET LOCAL leaked onto the pool: SHOW statement_timeout = {shown_after}"
        ));
    }
    assert_statement_cancelled(&pool, "database statement_timeout=1s after cutover").await?;
    Ok(())
}

async fn prove_cap_rolls_back(admin: &str, db: &str) -> Result<(), String> {
    let url = provision(admin, db).await?;
    let pool = connect(&url).await?;
    if !prepare_graph(&pool).await? {
        eprintln!("SKIP issue494 rollback: Apache AGE is not available");
        return Ok(());
    }
    let graph = graph_name(&url, "issue494rb").await?;
    let (tenant, workspace) = insert_scope(&pool).await?;
    load_ring(
        &pool,
        &graph,
        tenant,
        workspace,
        ROLLBACK_NODES,
        ROLLBACK_EDGES,
    )
    .await?;
    let age_nodes = count_age(&pool, &graph, "Node").await?;
    let age_edges = count_age(&pool, &graph, "EDGE").await?;
    sqlx::query(
        "INSERT INTO public.eq_graph_nodes (namespace, node_id, properties)
         VALUES ('issue494rb', 'SEED_KEEP', '{}'::jsonb)",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let _clear = ClearCutoverTimeout;
    std::env::set_var("EDGEQUAKE_CUTOVER_STATEMENT_TIMEOUT", "50ms");
    let err = finish_plain_graph_cutover(&pool)
        .await
        .err()
        .ok_or("50ms cap did not cancel the copy; the fixture is too small")?;
    let text = err.to_string();
    eprintln!("issue494 capped cutover: {text}");
    for needle in [
        "statement_timeout",
        "rolled back",
        "backfilling",
        "edgequake migrate",
    ] {
        if !text.contains(needle) {
            return Err(format!("capped error missing {needle:?}: {text}"));
        }
    }
    if seed_count(&pool).await? != 1 {
        return Err("seed row was not restored; the copy transaction did not roll back".into());
    }
    if count_age(&pool, &graph, "Node").await? != age_nodes
        || count_age(&pool, &graph, "EDGE").await? != age_edges
    {
        return Err("AGE label counts changed during the failed copy".into());
    }
    if ledger_status(&pool).await?.as_deref() != Some("backfilling") {
        return Err(format!(
            "ledger after cancel: {:?}",
            ledger_status(&pool).await?
        ));
    }

    std::env::remove_var("EDGEQUAKE_CUTOVER_STATEMENT_TIMEOUT");
    let report = finish_plain_graph_cutover(&pool)
        .await
        .map_err(|e| format!("rerun after clearing the cap failed: {e}"))?;
    if !report.ready || report.mismatches != 0 {
        return Err(format!("rerun did not become ready: {report:?}"));
    }
    let plain_nodes = count_plain(&pool, "issue494rb", "eq_graph_nodes").await?;
    if plain_nodes != ROLLBACK_NODES {
        return Err(format!(
            "rerun plain nodes {plain_nodes}, expected {ROLLBACK_NODES}"
        ));
    }
    Ok(())
}

struct ClearCutoverTimeout;

impl Drop for ClearCutoverTimeout {
    fn drop(&mut self) {
        std::env::remove_var("EDGEQUAKE_CUTOVER_STATEMENT_TIMEOUT");
    }
}

async fn prepare_graph(pool: &PgPool) -> Result<bool, String> {
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    run_postgres_expandable_migrations(pool)
        .await
        .map_err(|e| format!("migrations failed: {e}"))?;
    let age_on: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
    if !age_on {
        return Ok(false);
    }
    sqlx::query(
        "UPDATE public.eq_graph_cutover
         SET status = 'age', age_absent = false, shadow_ok = false,
             backfill_complete = false, updated_at = now()
         WHERE id = 1",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(true)
}

async fn graph_name(url: &str, namespace: &str) -> Result<String, String> {
    let (host, port, user, password, db) = parse_url(url)?;
    let config = PostgresConfig::new(host, port, db, user, password)
        .with_namespace(namespace)
        .with_max_connections(2);
    let storage = PostgresAGEGraphStorage::new(config);
    storage
        .initialize()
        .await
        .map_err(|e| format!("initialize {namespace} failed: {e}"))?;
    Ok(storage.graph_name().to_string())
}

async fn insert_scope(pool: &PgPool) -> Result<(Uuid, Uuid), String> {
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    sqlx::query("INSERT INTO public.tenants (tenant_id, name, slug) VALUES ($1, 'issue494', $2)")
        .bind(tenant)
        .bind(format!("t{}", &tenant.simple().to_string()[..12]))
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO public.workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, 'issue494', $3)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(format!("w{}", &workspace.simple().to_string()[..12]))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok((tenant, workspace))
}

async fn load_ring(
    pool: &PgPool,
    graph: &str,
    tenant: Uuid,
    workspace: Uuid,
    nodes: i64,
    edges: i64,
) -> Result<(), String> {
    if !safe_ident(graph) {
        return Err(format!("unsafe graph name {graph}"));
    }
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;
    sqlx::query("SET statement_timeout = 0")
        .execute(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(&format!(
        r#"ALTER TABLE {graph}."Node" DISABLE TRIGGER USER"#
    ))
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("disable node triggers: {e}"))?;
    sqlx::query(&format!(
        r#"ALTER TABLE {graph}."EDGE" DISABLE TRIGGER USER"#
    ))
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("disable edge triggers: {e}"))?;

    let (node_id, node_seq): (i64, String) = sqlx::query_as(
        "SELECT l.id::bigint, l.seq_name::text
         FROM ag_catalog.ag_label l
         JOIN ag_catalog.ag_graph g ON l.graph = g.graphid
         WHERE g.name = $1 AND l.name = 'Node'",
    )
    .bind(graph)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| format!("node label: {e}"))?;
    let (edge_id, edge_seq): (i64, String) = sqlx::query_as(
        "SELECT l.id::bigint, l.seq_name::text
         FROM ag_catalog.ag_label l
         JOIN ag_catalog.ag_graph g ON l.graph = g.graphid
         WHERE g.name = $1 AND l.name = 'EDGE'",
    )
    .bind(graph)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| format!("edge label: {e}"))?;

    let inserted = sqlx::query(&format!(
        r#"INSERT INTO {graph}."Node" (id, properties, eq_node_id)
           SELECT ((($1::bigint << 48) | g)::text)::ag_catalog.graphid,
                  jsonb_build_object(
                    'node_id', 'N' || g::text,
                    'tenant_id', $2::text,
                    'workspace_id', $3::text,
                    'entity_type', 'Entity',
                    'description', repeat('x', 240)
                  )::text::ag_catalog.agtype,
                  'N' || g::text
           FROM generate_series(1, $4::bigint) AS g"#
    ))
    .bind(node_id)
    .bind(tenant.to_string())
    .bind(workspace.to_string())
    .bind(nodes)
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("insert nodes: {e}"))?;
    if inserted.rows_affected() != nodes as u64 {
        return Err(format!(
            "inserted {} nodes, expected {nodes}",
            inserted.rows_affected()
        ));
    }

    let inserted = sqlx::query(&format!(
        r#"INSERT INTO {graph}."EDGE"
             (id, start_id, end_id, properties, eq_source_id, eq_target_id, eq_rel_type)
           SELECT ((($1::bigint << 48) | s.g)::text)::ag_catalog.graphid,
                  sn.id,
                  tn.id,
                  jsonb_build_object(
                    'source_id', 'N' || s.src::text,
                    'target_id', 'N' || s.tgt::text,
                    'relation_type', s.rel,
                    'tenant_id', $2::text,
                    'workspace_id', $3::text,
                    'description', repeat('y', 80)
                  )::text::ag_catalog.agtype,
                  'N' || s.src::text,
                  'N' || s.tgt::text,
                  s.rel
           FROM (
             SELECT g,
                    ((g - 1) % $4) + 1 AS src,
                    (g % $4) + 1 AS tgt,
                    'R' || ((g - 1) / $4)::text AS rel
             FROM generate_series(1, $5::bigint) AS g
           ) s
           JOIN {graph}."Node" sn ON sn.eq_node_id = 'N' || s.src::text
           JOIN {graph}."Node" tn ON tn.eq_node_id = 'N' || s.tgt::text"#
    ))
    .bind(edge_id)
    .bind(tenant.to_string())
    .bind(workspace.to_string())
    .bind(nodes)
    .bind(edges)
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("insert edges: {e}"))?;
    if inserted.rows_affected() != edges as u64 {
        return Err(format!(
            "inserted {} edges, expected {edges}",
            inserted.rows_affected()
        ));
    }

    sqlx::query("SELECT setval(format('%I.%I', $1::text, $2::text)::regclass, $3::bigint)")
        .bind(graph)
        .bind(&node_seq)
        .bind(nodes)
        .execute(&mut *conn)
        .await
        .map_err(|e| format!("node sequence: {e}"))?;
    sqlx::query("SELECT setval(format('%I.%I', $1::text, $2::text)::regclass, $3::bigint)")
        .bind(graph)
        .bind(&edge_seq)
        .bind(edges)
        .execute(&mut *conn)
        .await
        .map_err(|e| format!("edge sequence: {e}"))?;
    sqlx::query(&format!(
        r#"ALTER TABLE {graph}."Node" ENABLE TRIGGER USER"#
    ))
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("enable node triggers: {e}"))?;
    sqlx::query(&format!(
        r#"ALTER TABLE {graph}."EDGE" ENABLE TRIGGER USER"#
    ))
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("enable edge triggers: {e}"))?;
    Ok(())
}

async fn assert_statement_cancelled(pool: &PgPool, label: &str) -> Result<(), String> {
    match sqlx::query("SELECT pg_sleep(2)").execute(pool).await {
        Ok(_) => Err(format!("{label}: pg_sleep(2) was not cancelled")),
        Err(err) => {
            let text = err.to_string();
            if text.contains("statement timeout") || text.contains("57014") {
                Ok(())
            } else {
                Err(format!("{label}: unexpected cancel error: {text}"))
            }
        }
    }
}

async fn show_statement_timeout(pool: &PgPool) -> Result<String, String> {
    sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())
}

async fn count_plain(pool: &PgPool, namespace: &str, table: &str) -> Result<i64, String> {
    if table != "eq_graph_nodes" && table != "eq_graph_edges" {
        return Err(format!("unexpected plain table {table}"));
    }
    sqlx::query_scalar(&format!(
        "SELECT COUNT(*)::bigint FROM public.{table} WHERE namespace = $1"
    ))
    .bind(namespace)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}

async fn count_age(pool: &PgPool, graph: &str, label: &str) -> Result<i64, String> {
    if !safe_ident(graph) || (label != "Node" && label != "EDGE") {
        return Err("unsafe AGE count target".into());
    }
    sqlx::query_scalar(&format!(
        r#"SELECT COUNT(*)::bigint FROM {graph}."{label}""#
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}

async fn ledger_status(pool: &PgPool) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT status FROM public.eq_graph_cutover WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

async fn seed_count(pool: &PgPool) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM public.eq_graph_nodes
         WHERE namespace = 'issue494rb' AND node_id = 'SEED_KEEP'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}

fn safe_ident(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

async fn connect(url: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .max_connections(4)
        .connect(url)
        .await
        .map_err(|e| e.to_string())
}

async fn provision(admin: &str, db: &str) -> Result<String, String> {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(&format!("CREATE DATABASE {db}"))
        .execute(&admin_pool)
        .await
        .map_err(|e| format!("create database {db}: {e}"))?;
    admin_pool.close().await;
    Ok(replace_db(admin, db))
}

async fn alter_database_timeout(admin: &str, db: &str, timeout: &str) -> Result<(), String> {
    if !safe_ident(db) || !matches!(timeout, "1s" | "0") {
        return Err(format!("refusing timeout alter {db} {timeout}"));
    }
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(&format!(
        "ALTER DATABASE {db} SET statement_timeout = '{timeout}'"
    ))
    .execute(&admin_pool)
    .await
    .map_err(|e| format!("alter database timeout: {e}"))?;
    admin_pool.close().await;
    Ok(())
}

async fn drop_db(admin: &str, db: &str) {
    if !safe_ident(db) {
        return;
    }
    let Ok(admin_pool) = PgPoolOptions::new().max_connections(1).connect(admin).await else {
        return;
    };
    sqlx::query(&format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db}' AND pid <> pg_backend_pid()"
    ))
    .execute(&admin_pool)
    .await
    .ok();
    sqlx::query(&format!("DROP DATABASE IF EXISTS {db}"))
        .execute(&admin_pool)
        .await
        .ok();
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

fn parse_url(url: &str) -> Result<(String, u16, String, String, String), String> {
    let without_scheme = url.split("://").nth(1).ok_or("scheme")?;
    let (auth, host_path) = without_scheme.split_once('@').ok_or("auth")?;
    let (user, password) = auth.split_once(':').ok_or("user")?;
    let (host_port, db_and_q) = host_path.split_once('/').ok_or("host")?;
    let db = db_and_q.split('?').next().unwrap_or(db_and_q);
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(5432)),
        None => (host_port.to_string(), 5432),
    };
    Ok((
        host,
        port,
        user.to_string(),
        password.to_string(),
        db.to_string(),
    ))
}
