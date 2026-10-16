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

fn dev_mode_enabled() -> bool {
    match std::env::var("EDGEQUAKE_DEV_MODE") {
        Ok(v) => {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// Rewrite `_sqlx_migrations.checksum` for known fossils that match the ledger.
///
/// - Production fossils (`dev_only = false`): always auto-accept (fleet + local).
/// - Dev-only fossils: auto-accept only when `EDGEQUAKE_DEV_MODE` is on (make_dev).
///   Production stays fail-closed so accidental local SQL never silently ships.
pub(crate) async fn repair_known_production_fossils(pool: &PgPool) -> Result<usize, sqlx::Error> {
    if !helpers::sqlx_migrations_table_exists(pool).await? {
        return Ok(0);
    }
    let manifest = edgequake_migrate_manifest::load();
    let accept_dev_only = dev_mode_enabled();
    let mut repaired = 0usize;
    for entry in &manifest.migration {
        let fossils: Vec<&str> = entry
            .fossils
            .iter()
            .filter(|f| !f.dev_only || accept_dev_only)
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
            "SELECT encode(checksum, 'hex') FROM public._sqlx_migrations \
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
            "UPDATE public._sqlx_migrations SET checksum = decode($1, 'hex') \
             WHERE version = $2 AND success = true \
               AND encode(checksum, 'hex') = $3",
        )
        .bind(&current_hex)
        .bind(entry.version)
        .bind(&stored)
        .execute(pool)
        .await?;
        let kind = if entry
            .fossils
            .iter()
            .any(|f| f.dev_only && f.sha384.eq_ignore_ascii_case(stored.as_str()))
        {
            "dev_only"
        } else {
            "production"
        };
        info!(
            target: "edgequake.migration",
            version = entry.version,
            from = %stored,
            to = %current_hex,
            fossil_kind = kind,
            "Auto-accepted known fossil checksum (SPEC-150)"
        );
        repaired += 1;
    }
    Ok(repaired)
}
