//! Background CQRS entity backfill (migration 040).

use sqlx::PgPool;
use tracing::{info, warn};

use super::super::SQL_040_APPLY;

/// Background task: run the CQRS entity backfill (migration 040 apply.sql).
pub async fn reconcile_migration_040_background(pool: &PgPool) {
    let mode: Option<String> =
        sqlx::query_scalar("SELECT value::text FROM server_config WHERE key = 'entity_sync_mode'")
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    let mode_str = mode.as_deref().unwrap_or("\"disabled\"");
    if mode_str.contains("full") {
        info!(
            target: "edgequake.migration",
            step = "migration_040_skip",
            "CQRS backfill already complete (entity_sync_mode=full)"
        );
        return;
    }

    info!(
        target: "edgequake.migration",
        step = "migration_040_start",
        entity_sync_mode = %mode_str,
        "Starting CQRS entity backfill in background (SPEC-021 P2-02c)"
    );

    match sqlx::raw_sql(SQL_040_APPLY).execute(pool).await {
        Ok(_) => {
            info!(
                target: "edgequake.migration",
                step = "migration_040_complete",
                "CQRS entity backfill complete (entity_sync_mode=full)"
            );
        }
        Err(e) => {
            warn!(
                target: "edgequake.migration",
                step = "migration_040_failed",
                error = %e,
                "CQRS entity backfill failed — will retry on next restart"
            );
        }
    }
}
