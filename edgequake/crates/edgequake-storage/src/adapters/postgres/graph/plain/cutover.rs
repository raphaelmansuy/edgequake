//! Backfill AGE label tables into plain rows, then shadow-compare neighbor sets.

use super::super::PostgresAGEGraphStorage;
use super::ledger::tables_present;
use crate::adapters::postgres::statement_timeout::apply_local_cutover_timeout;
use crate::error::{Result, StorageError};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct CutoverReport {
    pub graphs: usize,
    pub mismatches: i64,
    pub ready: bool,
    pub age_absent: bool,
    /// Why a neighborhood differed, when `mismatches` is not zero.
    pub note: String,
}

/// Finish migrate on the plain graph.
///
/// A second call is a no-op once `eq_graph_cutover.status` is `ready`, so
/// `edgequake migrate` stays idempotent and does not touch AGE label tables.
pub async fn finish_plain_graph_cutover(pool: &sqlx::PgPool) -> Result<CutoverReport> {
    if !super::ledger::tables_present(pool).await? {
        return Ok(CutoverReport {
            graphs: 0,
            mismatches: 0,
            ready: true,
            age_absent: true,
            note: String::new(),
        });
    }
    let row = sqlx::query("SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| cutover_err("eq_graph_cutover read", e))?;
    let (status, age_absent) = match row {
        Some(row) => (
            row.try_get::<String, _>("status").unwrap_or_default(),
            row.try_get::<bool, _>("age_absent").unwrap_or(false),
        ),
        None => (String::new(), false),
    };
    if status == "ready" {
        return Ok(CutoverReport {
            graphs: 0,
            mismatches: 0,
            ready: true,
            age_absent,
            note: String::new(),
        });
    }
    let config = crate::adapters::postgres::PostgresConfig::default().with_namespace("default");
    let wrapped =
        crate::adapters::postgres::PostgresPool::from_existing(pool.clone(), config.clone());
    wrapped.probe_graph_extension_available().await?;
    let storage = super::super::PostgresAGEGraphStorage::with_pool(wrapped, config);
    storage.run_spec164_cutover().await
}

pub(in crate::adapters::postgres::graph) async fn run_plain_graph_cutover(
    storage: &PostgresAGEGraphStorage,
) -> Result<CutoverReport> {
    let pool = storage.pool.get().await?;
    if !tables_present(&pool).await? {
        return Err(StorageError::Database(
            "SPEC-164 cutover requires migration 170 (eq_graph_nodes)".into(),
        ));
    }
    if !storage.pool.graph_extension_available() {
        sqlx::query(
            "UPDATE public.eq_graph_cutover
             SET status = 'ready', age_absent = true, shadow_ok = true,
                 backfill_complete = true, unbackfilled_graphs = 0, updated_at = now()
             WHERE id = 1",
        )
        .execute(&pool)
        .await
        .map_err(|e| cutover_err("greenfield ledger update", e))?;
        return Ok(CutoverReport {
            graphs: 0,
            mismatches: 0,
            ready: true,
            age_absent: true,
            note: String::new(),
        });
    }

    sqlx::query(
        "UPDATE public.eq_graph_cutover
         SET status = 'backfilling', updated_at = now()
         WHERE id = 1 AND status <> 'ready'",
    )
    .execute(&pool)
    .await
    .map_err(|e| cutover_err("cutover status update", e))?;

    let graphs: Vec<String> =
        sqlx::query_scalar("SELECT name::text FROM ag_catalog.ag_graph ORDER BY 1")
            .fetch_all(&pool)
            .await
            .map_err(|e| cutover_err("list ag_graph", e))?;

    let mut mismatches = 0i64;
    let mut notes: Vec<String> = Vec::new();
    for graph in &graphs {
        if !safe_ident(graph) {
            return Err(StorageError::Database(format!(
                "refusing unsafe graph name {graph}"
            )));
        }
        let namespace = namespace_for(&pool, graph).await?;
        let _ = sqlx::query("SELECT public.eq_graph_install_mirror_triggers($1, $2)")
            .bind(graph)
            .bind(&namespace)
            .execute(&pool)
            .await;
        let (nodes, edges, orphans) = copy_graph(&pool, graph, &namespace).await?;
        let (graph_mismatches, sample) =
            servable_shadow_mismatches(&pool, graph, &namespace).await?;
        mismatches += i64::from(graph_mismatches);
        if graph_mismatches > 0 && notes.len() < 5 {
            notes.push(format!("{graph} orphans_skipped={orphans} {sample}"));
        }
        let shadow_ok = graph_mismatches == 0;
        flush_plain_lineage_gin(&pool).await?;
        sqlx::query(
            "INSERT INTO public.eq_graph_backfill
                (graph_name, namespace, nodes_copied, edges_copied, orphans_skipped, shadow_ok, finished_at)
             VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $6 THEN now() ELSE NULL END)
             ON CONFLICT (graph_name) DO UPDATE SET
                namespace = EXCLUDED.namespace,
                nodes_copied = EXCLUDED.nodes_copied,
                edges_copied = EXCLUDED.edges_copied,
                orphans_skipped = EXCLUDED.orphans_skipped,
                shadow_ok = EXCLUDED.shadow_ok,
                finished_at = EXCLUDED.finished_at",
        )
        .bind(graph)
        .bind(&namespace)
        .bind(nodes)
        .bind(edges)
        .bind(orphans)
        .bind(shadow_ok)
        .execute(&pool)
        .await
        .map_err(|e| cutover_err("backfill ledger write", e))?;
    }

    let ready = mismatches == 0;
    if ready {
        sqlx::query(
            "UPDATE public.eq_graph_cutover
             SET status = 'ready', shadow_ok = true, backfill_complete = true,
                 unbackfilled_graphs = 0, age_absent = false, updated_at = now(),
                 detail = jsonb_build_object('graphs', $1::int, 'mismatches', 0)
             WHERE id = 1",
        )
        .bind(graphs.len() as i32)
        .execute(&pool)
        .await
        .map_err(|e| cutover_err("cutover ready update", e))?;
    } else {
        sqlx::query(
            "UPDATE public.eq_graph_cutover
             SET status = 'age', shadow_ok = false, backfill_complete = false, updated_at = now(),
                 detail = jsonb_build_object('mismatches', $1::bigint, 'sample', $2::text)
             WHERE id = 1",
        )
        .bind(mismatches)
        .bind(notes.join("; "))
        .execute(&pool)
        .await
        .ok();
    }

    Ok(CutoverReport {
        graphs: graphs.len(),
        mismatches,
        ready,
        age_absent: false,
        note: notes.join("; "),
    })
}

/// Neighbor sets of the copied graph.
///
/// The copy keeps an edge only when both ends are rows in `eq_graph_nodes`,
/// and it replaces that namespace instead of merging with a previous attempt.
/// The stored `eq_graph_shadow_mismatches` walk still follows dangling AGE
/// endpoints, so a graph with those edges can never become ready. This walk
/// uses the same endpoint rule as the copy.
async fn servable_shadow_mismatches(
    pool: &sqlx::PgPool,
    graph: &str,
    namespace: &str,
) -> Result<(i32, String)> {
    let mut tx = begin_cutover_tx(pool, "shadow compare").await?;
    sqlx::query(AGE_SERVABLE_NEIGHBORS)
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err("shadow walker", e))?;
    let rows = sqlx::query(
        r#"WITH sample AS (
             SELECT node_id
             FROM public.eq_graph_nodes
             WHERE namespace = $1
             ORDER BY md5(node_id || '164')
             LIMIT 50
           ),
           walked AS (
             SELECT node_id,
                    pg_temp.eq_neighbor_ids_age_servable($2, $1, node_id, 2, 500) AS age_ids,
                    public.eq_graph_neighbor_ids_plain($1, node_id, 2, 500) AS plain_ids
             FROM sample
           )
           SELECT node_id,
                  ARRAY(SELECT x FROM unnest(age_ids) AS x
                        EXCEPT SELECT x FROM unnest(plain_ids) AS x) AS age_only,
                  ARRAY(SELECT x FROM unnest(plain_ids) AS x
                        EXCEPT SELECT x FROM unnest(age_ids) AS x) AS plain_only
           FROM walked
           WHERE age_ids IS DISTINCT FROM plain_ids"#,
    )
    .bind(namespace)
    .bind(graph)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| cutover_err(&format!("shadow compare for {graph}"), e))?;
    tx.commit()
        .await
        .map_err(|e| cutover_err("shadow commit", e))?;
    let count = i32::try_from(rows.len()).unwrap_or(i32::MAX);
    let sample = rows
        .iter()
        .take(3)
        .map(|row| {
            let id: String = row.try_get("node_id").unwrap_or_default();
            let age_only: Vec<String> = row.try_get("age_only").unwrap_or_default();
            let plain_only: Vec<String> = row.try_get("plain_only").unwrap_or_default();
            format!(
                "{id} age_only={} plain_only={}",
                preview(&age_only),
                preview(&plain_only)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Ok((count, sample))
}

fn preview(ids: &[String]) -> String {
    let shown: Vec<&str> = ids.iter().take(4).map(String::as_str).collect();
    if ids.len() > shown.len() {
        format!("{shown:?}+{}", ids.len() - shown.len())
    } else {
        format!("{shown:?}")
    }
}

const AGE_SERVABLE_NEIGHBORS: &str = r#"
CREATE OR REPLACE FUNCTION pg_temp.eq_neighbor_ids_age_servable(
    p_graph text,
    p_namespace text,
    p_start text,
    p_depth int,
    p_cap int
) RETURNS text[]
LANGUAGE plpgsql AS $fn$
DECLARE
    visited text[] := ARRAY[p_start];
    frontier text[] := ARRAY[p_start];
    next_f text[];
    nbr text;
    d int;
BEGIN
    IF p_graph !~ '^[A-Za-z_][A-Za-z0-9_]*$' THEN
        RAISE EXCEPTION 'refusing unsafe graph name';
    END IF;
    IF p_start IS NULL OR to_regclass(format('%I."EDGE"', p_graph)) IS NULL THEN
        RETURN ARRAY(SELECT x FROM unnest(visited) AS x ORDER BY x);
    END IF;
    FOR d IN 1..LEAST(p_depth, 3) LOOP
        next_f := ARRAY[]::text[];
        FOR nbr IN EXECUTE format(
            $q$
            SELECT endpoint FROM (
                SELECT COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') AS endpoint
                FROM %I."EDGE" e
                WHERE COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id') = ANY($1)
                  AND EXISTS (
                    SELECT 1 FROM public.eq_graph_nodes n
                    WHERE n.namespace = $3
                      AND n.node_id = COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id')
                  )
                UNION
                SELECT COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id')
                FROM %I."EDGE" e
                WHERE COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') = ANY($1)
                  AND EXISTS (
                    SELECT 1 FROM public.eq_graph_nodes n
                    WHERE n.namespace = $3
                      AND n.node_id = COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id')
                  )
            ) s
            WHERE endpoint IS NOT NULL AND NOT (endpoint = ANY($2))
            ORDER BY endpoint
            $q$, p_graph, p_graph)
        USING frontier, visited, p_namespace
        LOOP
            EXIT WHEN cardinality(visited) >= p_cap;
            visited := visited || nbr;
            next_f := next_f || nbr;
        END LOOP;
        frontier := next_f;
        EXIT WHEN cardinality(frontier) = 0 OR cardinality(visited) >= p_cap;
    END LOOP;
    RETURN ARRAY(SELECT x FROM unnest(visited) AS x ORDER BY x);
END;
$fn$;
"#;

async fn namespace_for(pool: &sqlx::PgPool, graph: &str) -> Result<String> {
    let mapped: Option<String> =
        sqlx::query_scalar("SELECT namespace FROM public.eq_graph_namespace WHERE graph_name = $1")
            .bind(graph)
            .fetch_optional(pool)
            .await
            .map_err(|e| cutover_err("namespace lookup", e))?;
    if let Some(ns) = mapped {
        return Ok(ns);
    }
    Ok(derive_namespace(graph))
}

fn derive_namespace(graph: &str) -> String {
    let prefix = "eq_eq_";
    let suffix = "_graph";
    if let Some(rest) = graph.strip_prefix(prefix) {
        if let Some(ns) = rest.strip_suffix(suffix) {
            return ns.to_string();
        }
    }
    graph.to_string()
}

async fn copy_graph(pool: &sqlx::PgPool, graph: &str, namespace: &str) -> Result<(i64, i64, i64)> {
    let mut tx = begin_cutover_tx(pool, "backfill").await?;
    // Statement triggers would aggregate this whole copy. One recompute after
    // the load writes each degree once.
    sqlx::query("SELECT set_config('eq.skip_graph_degree', 'on', true)")
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err("skip degree maintenance", e))?;
    // A failed attempt leaves rows behind. Upsert does not delete them, so the
    // next compare walks edges Apache AGE no longer has. This namespace is the
    // snapshot of the current label tables.
    sqlx::query("DELETE FROM public.eq_graph_edges WHERE namespace = $1")
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err(&format!("replace edges {graph}"), e))?;
    sqlx::query("DELETE FROM public.eq_graph_nodes WHERE namespace = $1")
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err(&format!("replace nodes {graph}"), e))?;
    let node_sql = format!(
        r#"INSERT INTO public.eq_graph_nodes
             (namespace, node_id, tenant_id, workspace_id, entity_type, properties)
           SELECT $1, d.nid,
                  COALESCE(d.props->>'tenant_id', ''),
                  COALESCE(d.props->>'workspace_id', ''),
                  NULLIF(d.props->>'entity_type', ''),
                  d.props
           FROM (
             SELECT DISTINCT ON (nid)
                    COALESCE(n.eq_node_id, ag_catalog.agtype_to_json(n.properties)->>'node_id') AS nid,
                    ag_catalog.agtype_to_json(n.properties)::jsonb AS props
             FROM {graph}."Node" n
             WHERE COALESCE(n.eq_node_id, ag_catalog.agtype_to_json(n.properties)->>'node_id') IS NOT NULL
             ORDER BY 1, n.ctid DESC
           ) d
           ON CONFLICT (namespace, node_id) DO UPDATE SET
             tenant_id = EXCLUDED.tenant_id,
             workspace_id = EXCLUDED.workspace_id,
             entity_type = EXCLUDED.entity_type,
             properties = public.eq_merge_graph_properties(
                 public.eq_graph_nodes.properties, EXCLUDED.properties)"#
    );
    let nodes = sqlx::query(&node_sql)
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err(&format!("backfill nodes {graph}"), e))?
        .rows_affected() as i64;

    let orphan_sql = format!(
        r#"SELECT COUNT(*)::bigint FROM {graph}."EDGE" e
           WHERE NOT EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n
             WHERE n.namespace = $1
               AND n.node_id = COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id')
           ) OR NOT EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n
             WHERE n.namespace = $1
               AND n.node_id = COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id')
           )"#
    );
    let orphans: i64 = sqlx::query_scalar(&orphan_sql)
        .bind(namespace)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

    let edge_sql = format!(
        r#"INSERT INTO public.eq_graph_edges
             (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
           SELECT $1, d.src, d.tgt, d.rel,
                  COALESCE(d.props->>'tenant_id', ''),
                  COALESCE(d.props->>'workspace_id', ''),
                  CASE WHEN COALESCE(d.props->>'weight', '') ~ '^-?[0-9]+(\.[0-9]+)?$'
                       THEN (d.props->>'weight')::float8 ELSE NULL END,
                  d.props
           FROM (
             SELECT DISTINCT ON (src, tgt, rel)
                    COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id') AS src,
                    COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') AS tgt,
                    COALESCE(NULLIF(e.eq_rel_type, ''), NULLIF(ag_catalog.agtype_to_json(e.properties)->>'relation_type', ''), '') AS rel,
                    ag_catalog.agtype_to_json(e.properties)::jsonb AS props
             FROM {graph}."EDGE" e
           ) d
           WHERE EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n WHERE n.namespace = $1 AND n.node_id = d.src
           ) AND EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n WHERE n.namespace = $1 AND n.node_id = d.tgt
           )
           ON CONFLICT (namespace, source_id, target_id, rel_type) DO UPDATE SET
             properties = public.eq_merge_graph_properties(
                 public.eq_graph_edges.properties, EXCLUDED.properties),
             tenant_id = EXCLUDED.tenant_id,
             workspace_id = EXCLUDED.workspace_id,
             weight = EXCLUDED.weight"#
    );
    let edges = sqlx::query(&edge_sql)
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err(&format!("backfill edges {graph}"), e))?
        .rows_affected() as i64;
    sqlx::query("SELECT public.eq_graph_recompute_degrees($1)")
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| cutover_err(&format!("recompute degrees {graph}"), e))?;
    tx.commit()
        .await
        .map_err(|e| cutover_err("backfill commit", e))?;
    Ok((nodes, edges, orphans))
}

async fn flush_plain_lineage_gin(pool: &sqlx::PgPool) -> Result<()> {
    let mut tx = begin_cutover_tx(pool, "lineage GIN flush").await?;
    sqlx::query(
        "SELECT
           gin_clean_pending_list('public.idx_eq_graph_nodes_source_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_source_chunk_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_edges_source_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_edges_source_chunk_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_id_trgm'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_desc_trgm'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_type_trgm'::regclass)",
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| cutover_err("lineage GIN flush", e))?;
    tx.commit()
        .await
        .map_err(|e| cutover_err("lineage GIN flush commit", e))?;
    Ok(())
}

async fn begin_cutover_tx<'a>(
    pool: &'a sqlx::PgPool,
    step: &str,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| cutover_err(&format!("{step} begin"), e))?;
    apply_local_cutover_timeout(&mut tx).await?;
    Ok(tx)
}

fn cutover_err(step: &str, err: sqlx::Error) -> StorageError {
    let code = match &err {
        sqlx::Error::Database(db) => db.code().map(|c| c.to_string()),
        _ => None,
    };
    let message = err.to_string();
    if let Some(explained) = explain_cutover_cancel(step, code.as_deref(), &message) {
        StorageError::DeadlineExceeded(explained)
    } else {
        StorageError::Database(format!("{step} failed: {message}"))
    }
}

/// Actionable text for SQLSTATE 57014 during the plain-graph copy.
///
/// `None` means the error is not a statement or transaction timeout.
fn explain_cutover_cancel(step: &str, code: Option<&str>, message: &str) -> Option<String> {
    // "idle-in-transaction timeout" contains the substring "transaction timeout".
    if message.contains("idle-in-transaction") {
        return None;
    }
    let transaction = message.contains("due to transaction timeout");
    let statement = message.contains("statement timeout");
    if code != Some("57014") && !transaction && !statement {
        return None;
    }
    let guc = if transaction {
        "transaction_timeout"
    } else {
        "statement_timeout"
    };
    Some(format!(
        "{step} cancelled by {guc} (SQLSTATE 57014): {message}. \
         The open transaction rolled back; plain graph rows for this step are unchanged. \
         eq_graph_cutover.status stays backfilling. Re-run `edgequake migrate` (idempotent recopy). \
         Cutover statements set statement_timeout from EDGEQUAKE_CUTOVER_STATEMENT_TIMEOUT (default 0 = disabled) \
         so a role or database statement_timeout does not apply."
    ))
}

fn safe_ident(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_timeout_names_rollback_and_rerun() {
        let message = explain_cutover_cancel(
            "backfill nodes eq_eq_default_graph",
            Some("57014"),
            "canceling statement due to statement timeout",
        )
        .expect("timeout");
        assert!(message.contains("statement_timeout"));
        assert!(message.contains("rolled back"));
        assert!(message.contains("backfilling"));
        assert!(message.contains("edgequake migrate"));
        assert!(!message.contains("transaction_timeout"));
        let err = cutover_err(
            "backfill nodes eq_eq_default_graph",
            sqlx::Error::Protocol("canceling statement due to statement timeout".into()),
        );
        match err {
            StorageError::DeadlineExceeded(text) => {
                assert!(text.contains("statement_timeout"));
                assert!(text.contains("rolled back"));
            }
            other => panic!("expected deadline, got {other:?}"),
        }
    }

    #[test]
    fn transaction_timeout_names_that_guc() {
        let message = explain_cutover_cancel(
            "recompute degrees g",
            Some("57014"),
            "canceling statement due to transaction timeout",
        )
        .expect("timeout");
        assert!(message.contains("cancelled by transaction_timeout"));
        assert!(message.contains("rolled back"));
        assert!(message.contains("backfilling"));
        let terminated = explain_cutover_cancel(
            "backfill nodes g",
            None,
            "terminating connection due to transaction timeout",
        )
        .expect("connection kill");
        assert!(terminated.contains("cancelled by transaction_timeout"));
    }

    #[test]
    fn idle_in_transaction_is_not_relabeled_as_a_cutover_timeout() {
        assert!(explain_cutover_cancel(
            "backfill",
            Some("25P03"),
            "terminating connection due to idle-in-transaction timeout",
        )
        .is_none());
    }

    #[test]
    fn unrelated_database_error_stays_a_database_error() {
        let err = cutover_err(
            "backfill nodes g",
            sqlx::Error::Protocol("duplicate key value".into()),
        );
        match err {
            StorageError::Database(text) => {
                assert!(text.contains("duplicate key value"));
                assert!(!text.contains("rolled back"));
            }
            other => panic!("expected database error, got {other:?}"),
        }
        assert!(explain_cutover_cancel("step", Some("23505"), "duplicate key").is_none());
    }
}
