//! Checksum repair + known fossil auto-accept (SPEC-150 WP-2).

use sqlx::PgPool;
use tracing::info;

use super::helpers;
use super::ledger::MIGRATOR;

pub use super::checksum_repair::{
    allow_checksum_repair, authorize_checksum_rewrite, is_known_fossil, is_known_production_fossil,
    known_checksum_repair_versions, parse_allow_checksum_repair_list, refuse_silent_repair_message,
    ALLOW_CHECKSUM_REPAIR_ENV,
};

/// SPEC-150: for every version with production fossils, if the ledger stores a
/// known fossil hash, rewrite it to the current embedded MIGRATOR checksum.
pub(crate) async fn repair_known_production_fossils(pool: &PgPool) -> Result<usize, sqlx::Error> {
    if !helpers::sqlx_migrations_table_exists(pool).await? {
        return Ok(0);
    }
    let manifest = edgequake_migrate_manifest::load();
    let mut repaired = 0usize;
    for entry in &manifest.migration {
        let fossils: Vec<&str> = entry
            .fossils
            .iter()
            .filter(|f| !f.dev_only)
            .map(|f| f.sha384.as_str())
            .collect();
        if fossils.is_empty() {
            continue;
        }
        let Some(mig) = MIGRATOR.iter().find(|m| m.version == entry.version) else {
            continue;
        };
        let current_hex = hex::encode(&*mig.checksum);
        let stored: Option<String> = sqlx::query_scalar(
            "SELECT encode(checksum, 'hex') FROM _sqlx_migrations \
             WHERE version = $1 AND success = true",
        )
        .bind(entry.version)
        .fetch_optional(pool)
        .await?;
        let Some(stored) = stored else {
            continue;
        };
        if stored.eq_ignore_ascii_case(&current_hex) {
            continue;
        }
        if !fossils
            .iter()
            .any(|h| h.eq_ignore_ascii_case(stored.as_str()))
        {
            continue;
        }
        sqlx::query(
            "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
             WHERE version = $2 AND success = true \
               AND encode(checksum, 'hex') = $3",
        )
        .bind(&current_hex)
        .bind(entry.version)
        .bind(&stored)
        .execute(pool)
        .await?;
        info!(
            target: "edgequake.migration",
            version = entry.version,
            from = %stored,
            to = %current_hex,
            "Auto-accepted known production fossil checksum (SPEC-150)"
        );
        repaired += 1;
    }
    Ok(repaired)
}
