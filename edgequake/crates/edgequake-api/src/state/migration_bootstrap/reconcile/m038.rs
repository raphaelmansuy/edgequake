use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::helpers::{large_graph_threshold, quote_schema, set_large_graph_threshold};
use super::super::{Migration038Report, MIGRATION_038_VERSION, SQL_038_APPLY};

/// NAMEDATALEN-safe index names (≤63 bytes) on AGE child label tables.
const M038_NODE_SOURCE_ID: &str = "idx_node_source_id_expr";
const M038_NODE_SOURCE_IDS_GIN: &str = "idx_node_source_ids_gin";
const M038_EDGE_SOURCE_IDS_GIN: &str = "idx_edge_source_ids_gin";

pub async fn reconcile_migration_038(
    pool: &PgPool,
    applied_this_run: &[i64],
) -> Result<Migration038Report, sqlx::Error> {
    if edgequake_storage::graph_cutover_is_ready(pool).await {
        info!(
            target: "edgequake.migration",
            step = "migration_038_skip",
            reason = "cutover_ready",
            "Skipping migration 038 index verify — plain graph ledger is ready"
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
        execute_bootstrap_apply_sql(pool, SQL_038_APPLY).await?;
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

struct GraphIndexAudit {
    graph_name: String,
    vertex_count: Option<i64>,
    missing_indexes: Vec<String>,
}

async fn index_exists(pool: &PgPool, schema: &str, index_name: &str) -> Result<bool, sqlx::Error> {
    // WHY: pg_indexes.indexname is type `name` (NAMEDATALEN=63). sqlx binds text;
    // cast to name so audit matches CREATE INDEX identifiers on PG16/17/18.
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM pg_indexes
            WHERE schemaname = $1 AND indexname = $2::name
        )",
    )
    .bind(schema)
    .bind(index_name)
    .fetch_one(pool)
    .await
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

        let node_tbl: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("{}.\"Node\"", graph_name))
            .fetch_one(pool)
            .await?;

        let edge_tbl: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("{}.\"EDGE\"", graph_name))
            .fetch_one(pool)
            .await?;

        let vertex_count = if node_tbl.is_some() {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*)::bigint FROM {}.\"Node\"",
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
            (node_tbl.is_some(), M038_NODE_SOURCE_ID),
            (node_tbl.is_some(), M038_NODE_SOURCE_IDS_GIN),
            (edge_tbl.is_some(), M038_EDGE_SOURCE_IDS_GIN),
        ];

        for (required, index_name) in expected {
            if !required {
                continue;
            }
            let exists = index_exists(pool, &graph_name, index_name).await?;
            if !exists {
                missing_indexes.push(index_name.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m038_index_names_fit_namedatalen() {
        for name in [
            M038_NODE_SOURCE_ID,
            M038_NODE_SOURCE_IDS_GIN,
            M038_EDGE_SOURCE_IDS_GIN,
        ] {
            assert!(
                name.len() <= 63,
                "index name {name} exceeds NAMEDATALEN ({} bytes)",
                name.len()
            );
        }
    }
}
