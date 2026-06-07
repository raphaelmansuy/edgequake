//! PostgreSQL migration bootstrap — SPEC-006 / SPEC-017 (SRP).
//!
//! First principle: sqlx records schema versions; **blocking DDL** runs only
//! size-aware in post-hooks (never in sqlx migrate for migration 038).

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::{info, warn};

/// Size-aware index DDL — SSOT: `migrations/support/038/apply.sql`
const SQL_038_APPLY: &str = include_str!("../../../../migrations/support/038/apply.sql");

/// sqlx migration version marker (no blocking DDL in sqlx file).
pub const MIGRATION_038_VERSION: i64 = 38;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Outcome of bootstrap migration run (surfaced in `/health` and `/ready`).
#[derive(Debug, Clone)]
pub struct MigrationBootstrapReport {
    pub pending_before: usize,
    pub applied_versions: Vec<i64>,
    pub latest_version: Option<i64>,
    pub migration_038: Migration038Report,
}

/// Post-sqlx status for migration 038 indexes.
#[derive(Debug, Clone)]
pub struct Migration038Report {
    pub age_available: bool,
    pub graphs_checked: usize,
    pub indexes_ready: bool,
    pub indexes_repaired_inline: bool,
    pub deferred_large_graphs: Vec<String>,
    pub missing_indexes: Vec<String>,
    pub operator_action: Option<String>,
}

impl Migration038Report {
    pub fn is_degraded(&self) -> bool {
        !self.indexes_ready && self.age_available
    }
}

/// True when the process may receive traffic (readiness probe).
pub fn is_ready_for_traffic(report: &Option<MigrationBootstrapReport>) -> bool {
    match report {
        None => true,
        Some(r) => !r.migration_038.is_degraded(),
    }
}

/// Run sqlx migrations plus size-aware 038 apply with structured progression logs.
pub async fn run_postgres_migrations(
    pool: &PgPool,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    info!(
        target: "edgequake.migration",
        step = "bootstrap_start",
        total_embedded = MIGRATOR.migrations.len(),
        "Database migration bootstrap starting"
    );

    let applied_before = fetch_applied_versions(pool).await?;
    let pending: Vec<_> = MIGRATOR
        .migrations
        .iter()
        .filter(|m| !applied_before.contains(&m.version))
        .collect();

    info!(
        target: "edgequake.migration",
        step = "preflight",
        applied = applied_before.len(),
        pending = pending.len(),
        latest_applied = applied_before.iter().max().copied(),
        "Migration preflight complete"
    );

    for (idx, migration) in pending.iter().enumerate() {
        info!(
            target: "edgequake.migration",
            step = "pending",
            progress = format!("{}/{}", idx + 1, pending.len()),
            version = migration.version,
            description = %migration.description,
            "Pending migration queued"
        );
    }

    if pending.is_empty() {
        info!(
            target: "edgequake.migration",
            step = "sqlx_run",
            "Schema up to date — no sqlx migrations to apply"
        );
    } else {
        info!(
            target: "edgequake.migration",
            step = "sqlx_run",
            count = pending.len(),
            "Applying sqlx migrations (advisory lock held)"
        );
        MIGRATOR.run(pool).await?;
        info!(
            target: "edgequake.migration",
            step = "sqlx_complete",
            count = pending.len(),
            "sqlx migrations applied successfully"
        );
    }

    let applied_after = fetch_applied_versions(pool).await?;
    let applied_this_run: Vec<i64> = applied_after
        .iter()
        .filter(|v| !applied_before.contains(v))
        .copied()
        .collect();

    for version in &applied_this_run {
        if let Some(m) = MIGRATOR.migrations.iter().find(|m| m.version == *version) {
            info!(
                target: "edgequake.migration",
                step = "applied",
                version = m.version,
                description = %m.description,
                "Migration applied in this bootstrap"
            );
        }
    }

    let migration_038 = reconcile_migration_038(pool, &applied_this_run).await?;

    if migration_038.is_degraded() {
        warn!(
            target: "edgequake.migration",
            step = "migration_038_degraded",
            missing = ?migration_038.missing_indexes,
            deferred = ?migration_038.deferred_large_graphs,
            action = migration_038.operator_action.as_deref().unwrap_or("none"),
            "Migration 038 indexes incomplete — /ready will fail until ops completes CONCURRENTLY apply"
        );
    } else if migration_038.indexes_repaired_inline {
        info!(
            target: "edgequake.migration",
            step = "migration_038_repaired",
            graphs = migration_038.graphs_checked,
            "Migration 038 indexes verified/repaired at bootstrap"
        );
    } else {
        info!(
            target: "edgequake.migration",
            step = "migration_038_ok",
            graphs = migration_038.graphs_checked,
            "Migration 038 indexes verified"
        );
    }

    info!(
        target: "edgequake.migration",
        step = "bootstrap_complete",
        latest_version = applied_after.iter().max().copied(),
        ready_for_traffic = !migration_038.is_degraded(),
        "Database migration bootstrap complete"
    );

    Ok(MigrationBootstrapReport {
        pending_before: pending.len(),
        applied_versions: applied_this_run,
        latest_version: applied_after.iter().max().copied(),
        migration_038,
    })
}

async fn fetch_applied_versions(pool: &PgPool) -> Result<HashSet<i64>, sqlx::Error> {
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = '_sqlx_migrations'
        )",
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(HashSet::new());
    }

    let rows: Vec<i64> = sqlx::query_scalar(
        "SELECT version FROM _sqlx_migrations WHERE success = true ORDER BY version",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

async fn set_large_graph_threshold(pool: &PgPool, threshold: i64) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('edgequake.migration_large_graph_threshold', $1, false)")
        .bind(threshold.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

async fn reconcile_migration_038(
    pool: &PgPool,
    applied_this_run: &[i64],
) -> Result<Migration038Report, sqlx::Error> {
    let age_available = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'age')",
    )
    .fetch_one(pool)
    .await?;

    if !age_available {
        info!(
            target: "edgequake.migration",
            step = "migration_038_skip",
            reason = "age_not_installed",
            "Skipping migration 038 index verify — Apache AGE not installed"
        );
        return Ok(Migration038Report {
            age_available: false,
            graphs_checked: 0,
            indexes_ready: true,
            indexes_repaired_inline: false,
            deferred_large_graphs: Vec::new(),
            missing_indexes: Vec::new(),
            operator_action: None,
        });
    }

    let threshold = large_graph_threshold();
    let mut graph_statuses = audit_graph_indexes(pool).await?;
    let graphs_checked = graph_statuses.len();
    let had_missing = graph_statuses.iter().any(|s| !s.missing_indexes.is_empty());
    let marker_just_applied = applied_this_run.contains(&MIGRATION_038_VERSION);

    if had_missing || marker_just_applied {
        info!(
            target: "edgequake.migration",
            step = "migration_038_apply_start",
            had_missing,
            marker_just_applied,
            threshold,
            "Running size-aware migration 038 apply"
        );
        set_large_graph_threshold(pool, threshold).await?;
        sqlx::query(SQL_038_APPLY).execute(pool).await?;
        graph_statuses = audit_graph_indexes(pool).await?;
    }

    let mut missing_indexes: Vec<String> = Vec::new();
    let mut deferred_large_graphs: Vec<String> = Vec::new();

    for status in &graph_statuses {
        if status.missing_indexes.is_empty() {
            info!(
                target: "edgequake.migration",
                step = "migration_038_graph_ok",
                graph = %status.graph_name,
                vertices = status.vertex_count,
                "All source_ids indexes present"
            );
            continue;
        }

        for idx in &status.missing_indexes {
            missing_indexes.push(format!("{}.{}", status.graph_name, idx));
        }

        let vertices = status.vertex_count.unwrap_or(0);
        if vertices >= threshold {
            deferred_large_graphs.push(format!("{} ({} vertices)", status.graph_name, vertices));
        }
    }

    let indexes_ready = missing_indexes.is_empty();
    let operator_action = if indexes_ready {
        None
    } else {
        Some(
            "Run: ./edgequake/scripts/migrations/apply_038.sh --apply --concurrent --yes"
                .to_string(),
        )
    };

    Ok(Migration038Report {
        age_available: true,
        graphs_checked,
        indexes_ready,
        indexes_repaired_inline: had_missing && indexes_ready,
        deferred_large_graphs,
        missing_indexes,
        operator_action,
    })
}

pub fn large_graph_threshold() -> i64 {
    std::env::var("EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500_000)
}

struct GraphIndexAudit {
    graph_name: String,
    vertex_count: Option<i64>,
    missing_indexes: Vec<String>,
}

async fn audit_graph_indexes(pool: &PgPool) -> Result<Vec<GraphIndexAudit>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct GraphRow {
        name: String,
    }

    let graphs: Vec<GraphRow> =
        sqlx::query_as("SELECT name FROM ag_catalog.ag_graph ORDER BY name")
            .fetch_all(pool)
            .await?;

    let mut results = Vec::new();

    for graph in graphs {
        let graph_name = graph.name;
        let idx_prefix = graph_name.replace('.', "_");

        let vertex_tbl: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("{}.\"_ag_label_vertex\"", graph_name))
            .fetch_one(pool)
            .await?;

        let edge_tbl: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("{}.\"_ag_label_edge\"", graph_name))
            .fetch_one(pool)
            .await?;

        let vertex_count = if vertex_tbl.is_some() {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*)::bigint FROM {}.\"_ag_label_vertex\"",
                quote_schema(&graph_name)
            ))
            .fetch_one(pool)
            .await?;
            Some(count)
        } else {
            None
        };

        let mut missing_indexes = Vec::new();
        let expected = [
            (
                vertex_tbl.is_some(),
                format!("idx_{idx_prefix}_vertex_source_id"),
            ),
            (
                vertex_tbl.is_some(),
                format!("idx_{idx_prefix}_vertex_source_ids_gin"),
            ),
            (
                edge_tbl.is_some(),
                format!("idx_{idx_prefix}_edge_source_ids_gin"),
            ),
        ];

        for (required, index_name) in expected {
            if !required {
                continue;
            }
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(
                    SELECT 1 FROM pg_indexes
                    WHERE schemaname = $1 AND indexname = $2
                )",
            )
            .bind(&graph_name)
            .bind(&index_name)
            .fetch_one(pool)
            .await?;

            if !exists {
                missing_indexes.push(index_name);
            }
        }

        results.push(GraphIndexAudit {
            graph_name,
            vertex_count,
            missing_indexes,
        });
    }

    Ok(results)
}

fn quote_schema(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_038_apply_sql_embedded() {
        assert!(SQL_038_APPLY.contains("source_ids_gin"));
        assert!(SQL_038_APPLY.contains("CREATE INDEX IF NOT EXISTS"));
        assert!(SQL_038_APPLY.contains("migration_large_graph_threshold"));
    }

    #[test]
    fn degraded_when_indexes_missing_with_age() {
        let report = Migration038Report {
            age_available: true,
            graphs_checked: 1,
            indexes_ready: false,
            indexes_repaired_inline: false,
            deferred_large_graphs: vec!["g (600000 vertices)".into()],
            missing_indexes: vec!["g.idx_g_vertex_source_ids_gin".into()],
            operator_action: Some("apply concurrent".into()),
        };
        assert!(report.is_degraded());
        assert!(!is_ready_for_traffic(&Some(MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(38),
            migration_038: report,
        })));
    }

    #[test]
    fn ready_when_no_age_or_indexes_ok() {
        assert!(is_ready_for_traffic(&None));
        let report = Migration038Report {
            age_available: true,
            graphs_checked: 1,
            indexes_ready: true,
            indexes_repaired_inline: false,
            deferred_large_graphs: vec![],
            missing_indexes: vec![],
            operator_action: None,
        };
        assert!(is_ready_for_traffic(&Some(MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(38),
            migration_038: report,
        })));
    }
}
