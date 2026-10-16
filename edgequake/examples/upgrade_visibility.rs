//! Post-upgrade graph visibility probe (GitHub #477).
//!
//! Uses the same pool construction as the API: `PostgresPool::from_existing`
//! then `probe_graph_extension_available`, then graph read/write.
//!
//! Prints one `PROOF ` JSON object on stdout.

use std::collections::HashMap;
use std::sync::Mutex;

use edgequake_storage::adapters::postgres::{
    PostgresAGEGraphStorage, PostgresConfig, PostgresPool,
};
use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::Layer;
use tracing_subscriber::prelude::*;

static SQL_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct SqlLayer;

impl<S> Layer<S> for SqlLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().target() != "sqlx::query" {
            return;
        }
        let mut visit = SqlVisit::default();
        event.record(&mut visit);
        let sql = visit
            .statement
            .filter(|text| !text.trim().is_empty())
            .or(visit.summary);
        if let Some(sql) = sql {
            if let Ok(mut guard) = SQL_LOG.lock() {
                guard.push(sql);
            }
        }
    }
}

#[derive(Default)]
struct SqlVisit {
    statement: Option<String>,
    summary: Option<String>,
}

impl Visit for SqlVisit {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_text(field, value);
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let text = format!("{value:?}");
        let text = unescape_debug(&text);
        self.record_text(field, &text);
    }
}

impl SqlVisit {
    fn record_text(&mut self, field: &Field, value: &str) {
        match field.name() {
            "db.statement" if !value.trim().is_empty() => self.statement = Some(value.to_string()),
            "summary" => self.summary = Some(value.to_string()),
            _ => {}
        }
    }
}

fn sql_len() -> usize {
    SQL_LOG.lock().map(|guard| guard.len()).unwrap_or(0)
}

fn sql_since(start: usize) -> Vec<String> {
    SQL_LOG
        .lock()
        .map(|guard| guard.iter().skip(start).cloned().collect())
        .unwrap_or_default()
}

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::registry().with(SqlLayer).try_init();
    match run().await {
        Ok(report) => {
            println!("PROOF {}", report);
            if report["pass"] == json!(true) {
                std::process::exit(0);
            }
            std::process::exit(2);
        }
        Err(err) => {
            let report = json!({
                "pass": false,
                "reasons": [err],
            });
            println!("PROOF {}", report);
            std::process::exit(2);
        }
    }
}

async fn run() -> Result<Value, String> {
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required".to_string())?;
    let fixture_node = std::env::var("FIXTURE_NODE").unwrap_or_else(|_| "PRE_A".into());
    let fixture_other = std::env::var("FIXTURE_OTHER").unwrap_or_else(|_| "PRE_B".into());
    let expect_fixture = std::env::var("EXPECT_FIXTURE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(true);
    let doc_id = std::env::var("DOC_ID").unwrap_or_else(|_| {
        format!(
            "00000000-0000-4000-8000-{:012x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(1)
                & 0xFFFF_FFFF_FFFF
        )
    });
    let new_node = format!("POST_{doc_id}");

    let raw = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .map_err(|e| format!("connect failed: {e}"))?;

    let age_node_before = count_age_node(&raw, &new_node).await;
    let age_edge_before = count_age_edge(&raw, &fixture_node, &new_node).await;
    let server_mark = sql_len();

    let config = config_from_url(&url)?;
    let wrapped = PostgresPool::from_existing(raw.clone(), config.clone());
    if wrapped.graph_extension_available() {
        return Err("from_existing must start with graph_extension_available false".into());
    }
    let probed = wrapped
        .probe_graph_extension_available()
        .await
        .map_err(|e| format!("AGE probe failed: {e}"))?;

    let storage = PostgresAGEGraphStorage::with_pool(wrapped.clone(), config);
    storage
        .initialize()
        .await
        .map_err(|e| format!("graph initialize failed: {e}"))?;

    let mut reasons: Vec<String> = Vec::new();

    let fixture_node_visible = match storage.get_node(&fixture_node).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            reasons.push(format!("fixture get_node failed: {e}"));
            false
        }
    };
    let fixture_edge_visible = match storage.get_edge(&fixture_node, &fixture_other).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            reasons.push(format!("fixture get_edge failed: {e}"));
            false
        }
    };
    if expect_fixture && !fixture_node_visible {
        reasons.push(format!(
            "pre-upgrade node {fixture_node} is not visible on the graph read path"
        ));
    }
    if expect_fixture && !fixture_edge_visible {
        reasons.push(format!(
            "pre-upgrade edge {fixture_node}->{fixture_other} is not visible on the graph read path"
        ));
    }

    let (document_inserted, document_error) = insert_document(&raw, &doc_id).await;

    let mut node_props = HashMap::new();
    node_props.insert("entity_type".into(), json!("PERSON"));
    node_props.insert(
        "description".into(),
        json!("entity written after migrate for a new document"),
    );
    node_props.insert("tenant_id".into(), json!("default"));
    node_props.insert("workspace_id".into(), json!("default"));
    node_props.insert("source_ids".into(), json!([doc_id]));
    node_props.insert(
        "source_chunk_ids".into(),
        json!([format!("{doc_id}-chunk-0")]),
    );
    node_props.insert("document_id".into(), json!(doc_id));
    if let Err(e) = storage.upsert_node(&new_node, node_props).await {
        reasons.push(format!("post-upgrade node upsert failed: {e}"));
    }

    let mut edge_props = HashMap::new();
    edge_props.insert("relation_type".into(), json!("MENTIONS"));
    edge_props.insert("weight".into(), json!(1.0));
    edge_props.insert("tenant_id".into(), json!("default"));
    edge_props.insert("workspace_id".into(), json!("default"));
    edge_props.insert("source_ids".into(), json!([doc_id]));
    edge_props.insert("description".into(), json!("post-upgrade document edge"));
    if let Err(e) = storage
        .upsert_edge(&fixture_node, &new_node, edge_props)
        .await
    {
        reasons.push(format!("post-upgrade edge upsert failed: {e}"));
    }

    let new_node_visible = match storage.get_node(&new_node).await {
        Ok(Some(node)) => {
            let sources = node
                .properties
                .get("source_ids")
                .cloned()
                .unwrap_or(Value::Null);
            if !sources.to_string().contains(&doc_id) {
                reasons.push(format!(
                    "new node is visible but source_ids do not contain document {doc_id}: {sources}"
                ));
            }
            true
        }
        Ok(None) => {
            reasons.push("new document entity is not visible on the graph read path".into());
            false
        }
        Err(e) => {
            reasons.push(format!("new get_node failed: {e}"));
            false
        }
    };
    let new_edge_visible = match storage.get_edge(&fixture_node, &new_node).await {
        Ok(Some(_)) => true,
        Ok(None) => {
            reasons.push("new document edge is not visible on the graph read path".into());
            false
        }
        Err(e) => {
            reasons.push(format!("new get_edge failed: {e}"));
            false
        }
    };

    let ledger = ledger_row(&raw).await;
    let ledger_status = ledger
        .as_ref()
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let server_statements = sql_since(server_mark);
    let new_node_in_age = count_age_node(&raw, &new_node).await;
    let new_node_in_plain = count_plain_node(&raw, &new_node).await;
    let new_edge_in_age = count_age_edge(&raw, &fixture_node, &new_node).await;
    let new_edge_in_plain = count_plain_edge(&raw, &fixture_node, &new_node).await;
    let explains = explain_graph_statements(&raw, &server_statements).await;

    let ledger_claims_age = ledger_status == "age";
    let age_write = new_node_in_age.unwrap_or(0) > 0 || new_edge_in_age.unwrap_or(0) > 0;
    let plain_write = new_node_in_plain.unwrap_or(0) > 0 && new_edge_in_plain.unwrap_or(0) > 0;
    let writes_plain_only = plain_write && !age_write;

    if ledger_status != "ready" {
        reasons.push(format!(
            "ledger status is {ledger_status:?}; migrate must finish on the plain graph"
        ));
    }
    if probed {
        reasons.push(
            "boot probe left graph_extension_available true after migrate (ingest_age/query_age)"
                .into(),
        );
    }
    if age_node_before.unwrap_or(0) != new_node_in_age.unwrap_or(0)
        || age_edge_before.unwrap_or(0) != new_edge_in_age.unwrap_or(0)
    {
        reasons.push(format!(
            "AGE label counts for the new document changed: node {age_node_before:?}->{new_node_in_age:?} edge {age_edge_before:?}->{new_edge_in_age:?}"
        ));
    }
    if age_write {
        reasons.push("new document graph rows were written to Apache AGE label tables".into());
    }
    if !server_statements
        .iter()
        .any(|sql| sql.contains("eq_graph_nodes"))
    {
        reasons.push("server graph path did not touch eq_graph_nodes".into());
    }
    if !server_statements
        .iter()
        .any(|sql| sql.contains("eq_graph_edges"))
    {
        reasons.push("server graph path did not touch eq_graph_edges".into());
    }
    for sql in &server_statements {
        if statement_uses_age(sql) {
            reasons.push(format!("server statement uses Apache AGE: {sql}"));
        }
    }
    for (sql, plan) in &explains {
        if plan.starts_with("EXPLAIN failed:") {
            reasons.push(format!("{plan}\nstatement: {sql}"));
        } else if statement_uses_age(plan) {
            reasons.push(format!("EXPLAIN plan uses Apache AGE for: {sql}\n{plan}"));
        }
    }
    if explains.is_empty() {
        reasons.push("no EXPLAIN plan was captured for the plain graph statements".into());
    }
    if !new_node_visible {
        reasons.push("new document entity is not visible on the plain graph read path".into());
    }
    if ledger_status == "ready" && new_node_in_plain.unwrap_or(0) == 0 {
        reasons.push(
            "ledger status is ready but the new document entity is absent from plain graph tables"
                .into(),
        );
    }
    if ledger_status == "ready" && new_edge_in_plain.unwrap_or(0) == 0 {
        reasons.push(
            "ledger status is ready but the new document edge is absent from eq_graph_edges".into(),
        );
    }
    if !document_inserted {
        reasons.push(format!(
            "documents row was not inserted: {}",
            document_error.unwrap_or_else(|| "unknown".into())
        ));
    }

    let report = json!({
        "pass": reasons.is_empty(),
        "reasons": reasons,
        "graph_extension_available": probed,
        "ledger": ledger,
        "ledger_status": if ledger_status.is_empty() { Value::Null } else { json!(ledger_status) },
        "ledger_claims_age": ledger_claims_age,
        "writes_plain_only": writes_plain_only,
        "fixture_node": fixture_node,
        "fixture_node_visible": fixture_node_visible,
        "fixture_edge_visible": fixture_edge_visible,
        "expect_fixture": expect_fixture,
        "document_id": doc_id,
        "document_inserted": document_inserted,
        "new_node": new_node,
        "new_node_visible": new_node_visible,
        "new_edge_visible": new_edge_visible,
        "new_node_in_age": new_node_in_age,
        "new_node_in_plain": new_node_in_plain,
        "new_edge_in_age": new_edge_in_age,
        "new_edge_in_plain": new_edge_in_plain,
        "age_node_before": age_node_before,
        "age_edge_before": age_edge_before,
        "server_statements": server_statements,
        "explains": explains.iter().map(|(sql, plan)| json!({"sql": sql, "plan": plan})).collect::<Vec<_>>(),
    });
    Ok(report)
}

fn config_from_url(url: &str) -> Result<PostgresConfig, String> {
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .ok_or_else(|| format!("unsupported DATABASE_URL scheme: {url}"))?;
    let (auth, hostpath) = rest
        .split_once('@')
        .ok_or_else(|| "DATABASE_URL is missing userinfo".to_string())?;
    let (user, password) = auth
        .split_once(':')
        .ok_or_else(|| "DATABASE_URL is missing a password".to_string())?;
    let (hostport, database) = hostpath
        .split_once('/')
        .ok_or_else(|| "DATABASE_URL is missing a database name".to_string())?;
    let database = database.split('?').next().unwrap_or(database);
    let (host, port) = match hostport.split_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>()
                .map_err(|_| format!("bad port in DATABASE_URL: {port}"))?,
        ),
        None => (hostport, 5432u16),
    };
    Ok(PostgresConfig::new(host, port, database, user, password).with_namespace("default"))
}

async fn ledger_row(pool: &sqlx::PgPool) -> Option<Value> {
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('public.eq_graph_cutover') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !present {
        return None;
    }
    let row = sqlx::query(
        "SELECT status, age_absent, shadow_ok, backfill_complete, unbackfilled_graphs, detail \
         FROM public.eq_graph_cutover WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .ok()??;
    let detail: Value = row.try_get("detail").unwrap_or(Value::Null);
    Some(json!({
        "status": row.try_get::<String, _>("status").ok(),
        "age_absent": row.try_get::<bool, _>("age_absent").ok(),
        "shadow_ok": row.try_get::<bool, _>("shadow_ok").ok(),
        "backfill_complete": row.try_get::<bool, _>("backfill_complete").ok(),
        "unbackfilled_graphs": row.try_get::<i32, _>("unbackfilled_graphs").ok(),
        "detail": detail,
    }))
}

async fn age_label_present(pool: &sqlx::PgPool, label: &str) -> bool {
    let rel = format!("eq_eq_default_graph.\"{label}\"");
    sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(rel)
        .fetch_one(pool)
        .await
        .unwrap_or(false)
}

async fn count_age_node(pool: &sqlx::PgPool, node_id: &str) -> Option<i64> {
    if !age_label_present(pool, "Node").await {
        return Some(0);
    }
    let sql = r#"
        SELECT count(*)::bigint
        FROM eq_eq_default_graph."Node" n
        WHERE ag_catalog.agtype_to_json(n.properties)->>'node_id' = $1
    "#;
    sqlx::query_scalar(sql)
        .bind(node_id)
        .fetch_one(pool)
        .await
        .ok()
}

async fn count_age_edge(pool: &sqlx::PgPool, source: &str, target: &str) -> Option<i64> {
    if !age_label_present(pool, "EDGE").await {
        return Some(0);
    }
    let sql = r#"
        SELECT count(*)::bigint
        FROM eq_eq_default_graph."EDGE" e
        WHERE ag_catalog.agtype_to_json(e.properties)->>'source_id' = $1
          AND ag_catalog.agtype_to_json(e.properties)->>'target_id' = $2
    "#;
    sqlx::query_scalar(sql)
        .bind(source)
        .bind(target)
        .fetch_one(pool)
        .await
        .ok()
}

async fn count_plain_node(pool: &sqlx::PgPool, node_id: &str) -> Option<i64> {
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('public.eq_graph_nodes') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !present {
        return Some(0);
    }
    sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.eq_graph_nodes WHERE namespace = 'default' AND node_id = $1",
    )
    .bind(node_id)
    .fetch_one(pool)
    .await
    .ok()
}

async fn count_plain_edge(pool: &sqlx::PgPool, source: &str, target: &str) -> Option<i64> {
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('public.eq_graph_edges') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !present {
        return Some(0);
    }
    sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.eq_graph_edges \
         WHERE namespace = 'default' AND source_id = $1 AND target_id = $2",
    )
    .bind(source)
    .bind(target)
    .fetch_one(pool)
    .await
    .ok()
}

async fn insert_document(pool: &sqlx::PgPool, doc_id: &str) -> (bool, Option<String>) {
    if let Err(err) = ensure_scope_row(pool, "tenants", "tenant_id").await {
        return (false, Some(err));
    }
    if let Err(err) = ensure_scope_row(pool, "workspaces", "workspace_id").await {
        return (false, Some(err));
    }
    let columns = match table_columns(pool, "documents").await {
        Ok(columns) => columns,
        Err(err) => return (false, Some(err)),
    };
    if columns.is_empty() {
        return (
            false,
            Some("public.documents does not exist after migrate".into()),
        );
    }

    let mut names: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();
    for (name, data_type, nullable, default) in &columns {
        if default.is_some()
            && name != "id"
            && name != "title"
            && name != "content"
            && name != "status"
        {
            continue;
        }
        let expr = match name.as_str() {
            "id" | "document_id" => {
                let cast = data_type_cast(data_type);
                format!("'{doc_id}'::{cast}")
            }
            "title" => "'upgrade-proof'".into(),
            "content" | "body" | "text" => "'post-upgrade document'".into(),
            "status" => match data_type.as_str() {
                "text" | "character varying" => "'indexed'".into(),
                _ => continue,
            },
            "content_hash" => "'proof'".into(),
            "metadata" | "extra" => "'{}'::jsonb".into(),
            "tenant_id" | "workspace_id" => tenant_expr(data_type),
            "created_at" | "updated_at" => "now()".into(),
            "chunk_count" | "entity_count" | "relationship_count" | "file_size_bytes"
            | "processing_time_ms" | "token_count" => "0".into(),
            _ if nullable == "YES" => continue,
            _ if default.is_some() => continue,
            _ => match data_type.as_str() {
                "uuid" => "gen_random_uuid()".into(),
                "text" | "character varying" => "'proof'".into(),
                "jsonb" | "json" => "'{}'::jsonb".into(),
                "boolean" => "false".into(),
                "integer" | "bigint" | "smallint" => "0".into(),
                "double precision" | "real" | "numeric" => "0".into(),
                "timestamp with time zone" | "timestamp without time zone" => "now()".into(),
                other => {
                    return (
                        false,
                        Some(format!("no filler for required documents.{name} ({other})")),
                    );
                }
            },
        };
        names.push(quote_ident(name));
        values.push(expr);
    }
    let sql = format!(
        "INSERT INTO public.documents ({}) VALUES ({})",
        names.join(", "),
        values.join(", ")
    );
    match sqlx::query(&sql).execute(pool).await {
        Ok(_) => (true, None),
        Err(e) => (false, Some(format!("{e}; sql={sql}"))),
    }
}

async fn ensure_scope_row(pool: &sqlx::PgPool, table: &str, id_column: &str) -> Result<(), String> {
    let columns = table_columns(pool, table).await?;
    if columns.is_empty() {
        return Ok(());
    }
    let mut names: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();
    for (name, data_type, nullable, default) in &columns {
        if default.is_some()
            && name != id_column
            && name != "name"
            && name != "slug"
            && name != "tenant_id"
        {
            continue;
        }
        let expr = match name.as_str() {
            "tenant_id" | "workspace_id" => tenant_expr(data_type),
            "name" => "'upgrade-proof'".into(),
            "slug" => "'upgrade-proof'".into(),
            "description" => "'upgrade proof scope'".into(),
            "settings" | "metadata" | "extra" => "'{}'::jsonb".into(),
            "created_at" | "updated_at" => "now()".into(),
            "is_active" => "true".into(),
            "plan" => "'free'".into(),
            _ if nullable == "YES" || default.is_some() => continue,
            _ => match data_type.as_str() {
                "uuid" => "gen_random_uuid()".into(),
                "text" | "character varying" => "'proof'".into(),
                "jsonb" | "json" => "'{}'::jsonb".into(),
                "boolean" => "false".into(),
                "integer" | "bigint" | "smallint" => "0".into(),
                "double precision" | "real" | "numeric" => "0".into(),
                "timestamp with time zone" | "timestamp without time zone" => "now()".into(),
                other => {
                    return Err(format!("no filler for required {table}.{name} ({other})"));
                }
            },
        };
        names.push(quote_ident(name));
        values.push(expr);
    }
    if names.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "INSERT INTO public.{table} ({}) VALUES ({}) ON CONFLICT ({id_column}) DO NOTHING",
        names.join(", "),
        values.join(", ")
    );
    sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| format!("{table} row was not inserted: {e}; sql={sql}"))?;
    Ok(())
}

async fn table_columns(
    pool: &sqlx::PgPool,
    table: &str,
) -> Result<Vec<(String, String, String, Option<String>)>, String> {
    let rows = sqlx::query(
        "SELECT column_name, data_type, is_nullable, column_default \
         FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 \
           AND is_generated = 'NEVER' \
         ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("{table} columns lookup failed: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.get("column_name"),
                row.get("data_type"),
                row.get("is_nullable"),
                row.try_get("column_default").ok(),
            )
        })
        .collect())
}

fn unescape_debug(text: &str) -> String {
    let trimmed = text.trim().trim_matches('"');
    let mut out = String::with_capacity(trimmed.len());
    let mut chars = trimmed.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn statement_uses_age(sql: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    lower.contains("ag_catalog")
        || lower.contains("agtype")
        || lower.contains("cypher")
        || sql.contains(".\"Node\"")
        || sql.contains(".\"EDGE\"")
}

async fn explain_graph_statements(
    pool: &sqlx::PgPool,
    statements: &[String],
) -> Vec<(String, String)> {
    let mut plans = Vec::new();
    for sql in statements {
        let trimmed = sql.trim();
        if trimmed.is_empty() || !is_graph_statement(trimmed) {
            continue;
        }
        let explain_sql = if trimmed.contains('$') {
            format!("EXPLAIN (FORMAT TEXT, GENERIC_PLAN) {trimmed}")
        } else {
            format!("EXPLAIN (FORMAT TEXT) {trimmed}")
        };
        match sqlx::raw_sql(&explain_sql).fetch_all(pool).await {
            Ok(rows) => {
                let plan = rows
                    .iter()
                    .filter_map(|row| row.try_get::<String, _>("QUERY PLAN").ok())
                    .collect::<Vec<_>>()
                    .join("\n");
                plans.push((trimmed.to_string(), plan));
            }
            Err(err) => plans.push((trimmed.to_string(), format!("EXPLAIN failed: {err}"))),
        }
    }
    plans
}

fn is_graph_statement(sql: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    (lower.contains("eq_graph_nodes") || lower.contains("eq_graph_edges"))
        && (lower.starts_with("select")
            || lower.starts_with("insert")
            || lower.starts_with("update")
            || lower.starts_with("delete")
            || lower.starts_with("with"))
}

fn data_type_cast(data_type: &str) -> &'static str {
    if data_type == "uuid" {
        "uuid"
    } else {
        "text"
    }
}

fn tenant_expr(data_type: &str) -> String {
    if data_type == "uuid" {
        "'00000000-0000-4000-8000-000000000001'::uuid".into()
    } else {
        "'default'".into()
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
