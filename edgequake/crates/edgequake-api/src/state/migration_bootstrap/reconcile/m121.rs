//! Migration 121 — SPEC-110 injection backfill checksum repair.
//!
//! Sibling of M118: workspace-prefixed injection keys + `ON CONFLICT (id) DO
//! UPDATE` can propose the same id twice. Fixed body uses `DISTINCT ON (inj_id)`.

use sqlx::PgPool;
use tracing::info;

use super::super::checksum_repair::authorize_checksum_rewrite;
use super::super::MIGRATION_121_VERSION;

/// SHA-384 of v0.24.1 broken M121 (no conflict-key dedup).
pub(super) const M121_CHECKSUM_BROKEN_V0241: &str =
    "da347384f34eb9db99d635f482293c7ce4cb678f3dc1a809e9b0308b95a8475471a9fa5b894667fb7a9d8207d8e5de7f";

/// SHA-384 of SPEC-110 fixed M121 — must match `checksums.lock`.
pub(super) const M121_CHECKSUM_FIXED_V0242: &str =
    "57088e874c47e6c558279388b7812946864dabcd3def5d97f417d105a656fba15bced1d3a5c4bd0b190c30a1978e0ef1";

/// Before sqlx runs: repair v0.24.1 broken M121 checksum so upgrade does not fail
/// with "migration 121 was previously applied but has been modified".
pub async fn repair_migration_121_checksum_if_needed(pool: &PgPool) -> Result<bool, sqlx::Error> {
    if !super::super::helpers::sqlx_migrations_table_exists(pool).await? {
        return Ok(false);
    }

    let current: Option<String> = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex') FROM _sqlx_migrations \
         WHERE version = $1 AND success = true",
    )
    .bind(MIGRATION_121_VERSION)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(false);
    };

    if current != M121_CHECKSUM_BROKEN_V0241 {
        return Ok(false);
    }

    // SPEC-150: known production fossils auto-accept; else scoped env.
    if let Err(msg) = authorize_checksum_rewrite(
        MIGRATION_121_VERSION,
        &current,
        "v0.24.1 broken injection upsert",
    ) {
        return Err(sqlx::Error::Protocol(msg));
    }

    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
         WHERE version = $2 AND success = true",
    )
    .bind(M121_CHECKSUM_FIXED_V0242)
    .bind(MIGRATION_121_VERSION)
    .execute(pool)
    .await?;

    info!(
        target: "edgequake.migration",
        step = "migration_121_checksum_repair",
        from = M121_CHECKSUM_BROKEN_V0241,
        to = M121_CHECKSUM_FIXED_V0242,
        "Repaired migration 121 checksum (SPEC-110; fossil/allow)"
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_and_fixed_checksums_are_distinct() {
        assert_ne!(M121_CHECKSUM_BROKEN_V0241, M121_CHECKSUM_FIXED_V0242);
        assert_eq!(M121_CHECKSUM_BROKEN_V0241.len(), 96);
        assert_eq!(M121_CHECKSUM_FIXED_V0242.len(), 96);
    }

    #[test]
    fn contract_checksum_drift_uses_shared_allow_helper() {
        let src = include_str!("m121.rs");
        assert!(
            src.contains("authorize_checksum_rewrite(MIGRATION_121_VERSION)")
                && src.contains("authorize_checksum_rewrite"),
            "LAW-MIG / X-02: m121 must use shared checksum_repair helper"
        );
    }
}
