//! Migration 078/079 — AGE child Node workspace index repair (SPEC-041 #273).
//!
//! First principle: migration 078 may be recorded without indexes (v0.13.2 skip-path)
//! or blocked before record (AGE+Node typo). Bootstrap reconcile ensures indexes
//! exist on every graph with a `"Node"` child table using SSOT apply SQL.

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::{info, warn};

use super::super::{MIGRATION_078_VERSION, MIGRATION_079_VERSION, SQL_078_APPLY};
use super::execute_bootstrap_apply_sql;

const CHILD_NODE_INDEXES: &[&str] = &[
    "idx_node_workspace_id",
    "idx_node_tenant_id",
    "idx_edge_start_id_text",
    "idx_edge_end_id_text",
];

/// SHA-384 of v0.13.2 broken M078 (invalid `->>>` operator).
pub(super) const M078_CHECKSUM_BROKEN_V0132: &str =
    "d22cc6d8416c6a8ccf28542c583724f139fcf70926edef4faec490b9daac665a7e526b9c46bdc758f4c05d0d0c1d62d2";

/// SHA-384 of v0.13.3+ fixed M078 — must match `checksums.lock`.
pub(super) const M078_CHECKSUM_FIXED_V0133: &str =
    "a043177271c82c65a7509855f1d64c02c46235343126a9bbb96c359f4c25aa35427c79bb50051d499b431d869eb8e930";

/// Before sqlx runs: repair v0.13.2 broken M078 checksum so upgrade does not fail
/// with "migration 78 was previously applied but has been modified".
pub async fn repair_migration_078_checksum_if_needed(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let current: Option<String> = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex') FROM _sqlx_migrations \
         WHERE version = $1 AND success = true",
    )
    .bind(MIGRATION_078_VERSION)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(false);
    };

    if current != M078_CHECKSUM_BROKEN_V0132 {
        return Ok(false);
    }

    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
         WHERE version = $2 AND success = true",
    )
    .bind(M078_CHECKSUM_FIXED_V0133)
    .bind(MIGRATION_078_VERSION)
    .execute(pool)
    .await?;

    info!(
        target: "edgequake.migration",
        step = "migration_078_checksum_repair",
        from = M078_CHECKSUM_BROKEN_V0132,
        to = M078_CHECKSUM_FIXED_V0133,
        "Repaired migration 078 checksum (SPEC-041 #273 v0.13.2 skip-path upgrade)"
    );

    Ok(true)
}

/// After sqlx: ensure child-table indexes exist when M078/M079 marker present.
pub async fn reconcile_migration_078(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
) -> Result<bool, sqlx::Error> {
    if !applied_after.contains(&MIGRATION_078_VERSION)
        && !applied_after.contains(&MIGRATION_079_VERSION)
    {
        return Ok(false);
    }

    let age_available = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'age')",
    )
    .fetch_one(pool)
    .await?;

    if !age_available {
        return Ok(false);
    }

    let missing = audit_missing_child_indexes(pool).await?;
    if missing.is_empty() {
        return Ok(false);
    }

    info!(
        target: "edgequake.migration",
        step = "migration_078_reconcile_start",
        missing_count = missing.len(),
        missing = ?missing,
        "Applying child Node index repair (support/078/apply.sql)"
    );

    execute_bootstrap_apply_sql(pool, SQL_078_APPLY).await?;

    let still_missing = audit_missing_child_indexes(pool).await?;
    if !still_missing.is_empty() {
        warn!(
            target: "edgequake.migration",
            step = "migration_078_reconcile_incomplete",
            still_missing = ?still_missing,
            "Some child indexes still missing after M078 reconcile"
        );
    }

    Ok(true)
}

async fn audit_missing_child_indexes(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
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
        let has_node: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(format!("{}.\"Node\"", graph.name))
            .fetch_one(pool)
            .await?;

        if !has_node {
            continue;
        }

        for index_name in CHILD_NODE_INDEXES {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(
                    SELECT 1 FROM pg_indexes WHERE schemaname = $1 AND indexname = $2
                )",
            )
            .bind(&graph.name)
            .bind(*index_name)
            .fetch_one(pool)
            .await?;

            if !exists {
                // Edge indexes only required when EDGE rel exists
                if index_name.starts_with("idx_edge_") {
                    let has_edge: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
                        .bind(format!("{}.\"EDGE\"", graph.name))
                        .fetch_one(pool)
                        .await?;
                    if !has_edge {
                        continue;
                    }
                }
                missing.push(format!("{}.{}", graph.name, index_name));
            }
        }
    }

    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_and_fixed_checksums_are_distinct() {
        assert_ne!(M078_CHECKSUM_BROKEN_V0132, M078_CHECKSUM_FIXED_V0133);
        assert_eq!(M078_CHECKSUM_BROKEN_V0132.len(), 96);
        assert_eq!(M078_CHECKSUM_FIXED_V0133.len(), 96);
    }
}
