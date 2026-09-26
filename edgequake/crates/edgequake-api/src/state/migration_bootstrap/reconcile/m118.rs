//! Migration 118 — SPEC-110 wsdoc backfill checksum repair.
//!
//! v0.24.1 shipped `SELECT DISTINCT` + `ON CONFLICT (id) DO UPDATE`, which fails
//! with Postgres 21000 when the same document_id appears under multiple wsdoc
//! workspace keys. The fixed body uses `DISTINCT ON (doc_id)`. Fleets that
//! already applied the broken body need a checksum rewrite before sqlx verify.

use sqlx::PgPool;
use tracing::info;

use super::super::checksum_repair::authorize_checksum_rewrite;
use super::super::MIGRATION_118_VERSION;

/// SHA-384 of v0.24.1 broken M118 (`SELECT DISTINCT` full-tuple upsert).
pub(super) const M118_CHECKSUM_BROKEN_V0241: &str =
    "331967467fdbeb58aeeb41ca92b6e3ec87ee84ace9286166275e14af9699a4cb862f1a92516043ee9c2489138a560629";

/// SHA-384 of SPEC-110 fixed M118 — must match `checksums.lock`.
pub(super) const M118_CHECKSUM_FIXED_V0242: &str =
    "a35e70d52e12215abe84283e4b0f853add44fb7ce9f2740f0673e840fdb385cb91eab4367bb0bf84c4c4894cdc370d9a";

/// Before sqlx runs: repair v0.24.1 broken M118 checksum so upgrade does not fail
/// with "migration 118 was previously applied but has been modified".
pub async fn repair_migration_118_checksum_if_needed(pool: &PgPool) -> Result<bool, sqlx::Error> {
    if !super::super::helpers::sqlx_migrations_table_exists(pool).await? {
        return Ok(false);
    }

    let current: Option<String> = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex') FROM _sqlx_migrations \
         WHERE version = $1 AND success = true",
    )
    .bind(MIGRATION_118_VERSION)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(false);
    };

    if current != M118_CHECKSUM_BROKEN_V0241 {
        return Ok(false);
    }

    // SPEC-150: known production fossils auto-accept; else scoped env.
    if let Err(msg) = authorize_checksum_rewrite(
        MIGRATION_118_VERSION,
        &current,
        "v0.24.1 broken wsdoc DISTINCT",
    ) {
        return Err(sqlx::Error::Protocol(msg));
    }

    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
         WHERE version = $2 AND success = true",
    )
    .bind(M118_CHECKSUM_FIXED_V0242)
    .bind(MIGRATION_118_VERSION)
    .execute(pool)
    .await?;

    info!(
        target: "edgequake.migration",
        step = "migration_118_checksum_repair",
        from = M118_CHECKSUM_BROKEN_V0241,
        to = M118_CHECKSUM_FIXED_V0242,
        "Repaired migration 118 checksum (SPEC-110; fossil/allow)"
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_and_fixed_checksums_are_distinct() {
        assert_ne!(M118_CHECKSUM_BROKEN_V0241, M118_CHECKSUM_FIXED_V0242);
        assert_eq!(M118_CHECKSUM_BROKEN_V0241.len(), 96);
        assert_eq!(M118_CHECKSUM_FIXED_V0242.len(), 96);
    }

    #[test]
    fn contract_checksum_drift_uses_shared_allow_helper() {
        let src = include_str!("m118.rs");
        assert!(
            src.contains("authorize_checksum_rewrite(MIGRATION_118_VERSION)")
                && src.contains("authorize_checksum_rewrite"),
            "LAW-MIG / X-02: m118 must use shared checksum_repair helper"
        );
    }
}
