//! Migration 046 — graph tenant isolation + query performance indexes.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::{info, warn};

use super::super::{Migration046Report, MIGRATION_046_VERSION, SQL_046_APPLY};

const REQUIRED_VERTEX_SUFFIXES: &[&str] = &[
    "_tenant_id",
    "_workspace_id",
    "_tenant_workspace",
    "_node_id",
];

const REQUIRED_EDGE_SUFFIXES: &[&str] =
    &["_edge_source_id", "_edge_target_id", "_ag_edge_start_id"];

pub async fn reconcile_migration_046(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration046Report, sqlx::Error> {
    let age_available = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'age')",
    )
    .fetch_one(pool)
    .await?;

    if !age_available {
        return Ok(Migration046Report {
            marker_present: applied_after.contains(&MIGRATION_046_VERSION),
            apply_executed: false,
            graphs_checked: 0,
            missing_indexes: Vec::new(),
        });
    }

    let marker_applied = applied_this_run.contains(&MIGRATION_046_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_046_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_046_apply_start",
        marker_applied,
        marker_present,
        "Ensuring graph tenant isolation perf indexes (migration 046)"
    );
    execute_bootstrap_apply_sql(pool, SQL_046_APPLY).await?;

    let missing = audit_required_indexes(pool).await?;
    if !missing.is_empty() {
        warn!(
            target: "edgequake.migration",
            step = "migration_046_incomplete",
            missing = ?missing,
            "Some graph perf indexes still missing after apply — check AGE graph tables exist"
        );
    }

    Ok(Migration046Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
        graphs_checked: count_graphs(pool).await?,
        missing_indexes: missing,
    })
}

async fn count_graphs(pool: &PgPool) -> Result<usize, sqlx::Error> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM ag_catalog.ag_graph")
        .fetch_one(pool)
        .await?;
    Ok(n as usize)
}

async fn audit_required_indexes(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct GraphRow {
        name: String,
    }

    let graphs: Vec<GraphRow> =
        sqlx::query_as("SELECT name FROM ag_catalog.ag_graph ORDER BY name")
            .fetch_all(pool)
            .await?;

    let mut missing = Vec::new();

    for graph in graphs {
        let idx_prefix = graph.name.replace('.', "_");
        let vertex_tbl: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("{}.\"_ag_label_vertex\"", graph.name))
            .fetch_one(pool)
            .await?;
        let edge_tbl: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("{}.\"_ag_label_edge\"", graph.name))
            .fetch_one(pool)
            .await?;

        if vertex_tbl.is_some() {
            for suffix in REQUIRED_VERTEX_SUFFIXES {
                let index_name = format!("idx_{idx_prefix}{suffix}");
                if !index_exists(pool, &graph.name, &index_name).await? {
                    missing.push(format!("{}.{}", graph.name, index_name));
                }
            }
        }

        if edge_tbl.is_some() {
            for suffix in REQUIRED_EDGE_SUFFIXES {
                let index_name = format!("idx_{idx_prefix}{suffix}");
                if !index_exists(pool, &graph.name, &index_name).await? {
                    missing.push(format!("{}.{}", graph.name, index_name));
                }
            }
        }
    }

    Ok(missing)
}

async fn index_exists(pool: &PgPool, schema: &str, index_name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM pg_indexes WHERE schemaname = $1 AND indexname = $2
        )",
    )
    .bind(schema)
    .bind(index_name)
    .fetch_one(pool)
    .await
}
