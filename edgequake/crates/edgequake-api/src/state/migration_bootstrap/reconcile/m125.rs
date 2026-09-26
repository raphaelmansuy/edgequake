//! Migration 125 — SPEC-111 KV residue cast-direction checksum repair.
//!
//! Pre-fix body cast indexed `uuid` columns to text (`d.id::text = substring(...)`),
//! defeating PK seeks. Fixed body casts the extracted key to uuid instead.

use sqlx::PgPool;
use tracing::info;

use super::super::checksum_repair::authorize_checksum_rewrite;
use super::super::MIGRATION_125_VERSION;

/// SHA-384 of pre-SPEC-111 M125 (indexed-column `::text` cast).
pub(super) const M125_CHECKSUM_BROKEN_PRE111: &str =
    "67b73fd0f683dd5cae06213ae59c75c2f8fea214074e8b250997aa77efc90a1fa01c14764f9fdb968b0e73685136b2f6";

/// SHA-384 of SPEC-111 fixed M125 — must match `checksums.lock`.
pub(super) const M125_CHECKSUM_FIXED_SPEC111: &str =
    "9ae99858a9c88ec9b0a195447d6f7e2601fb4423f0d846314b6aa06d337ad9e74e9a8998ae7359fba65df694d5b1eeec";

pub async fn repair_migration_125_checksum_if_needed(pool: &PgPool) -> Result<bool, sqlx::Error> {
    if !super::super::helpers::sqlx_migrations_table_exists(pool).await? {
        return Ok(false);
    }

    let current: Option<String> = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex') FROM _sqlx_migrations \
         WHERE version = $1 AND success = true",
    )
    .bind(MIGRATION_125_VERSION)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(false);
    };

    if current != M125_CHECKSUM_BROKEN_PRE111 {
        return Ok(false);
    }

    // SPEC-150: known production fossils auto-accept; else scoped env.
    if let Err(msg) = authorize_checksum_rewrite(
        MIGRATION_125_VERSION,
        &current,
        "SPEC-111 cast-direction fix",
    ) {
        return Err(sqlx::Error::Protocol(msg));
    }

    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
         WHERE version = $2 AND success = true",
    )
    .bind(M125_CHECKSUM_FIXED_SPEC111)
    .bind(MIGRATION_125_VERSION)
    .execute(pool)
    .await?;

    info!(
        target: "edgequake.migration",
        step = "migration_125_checksum_repair",
        from = M125_CHECKSUM_BROKEN_PRE111,
        to = M125_CHECKSUM_FIXED_SPEC111,
        "Repaired migration 125 checksum (SPEC-111; fossil/allow)"
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_and_fixed_checksums_are_distinct() {
        assert_ne!(M125_CHECKSUM_BROKEN_PRE111, M125_CHECKSUM_FIXED_SPEC111);
        assert_eq!(M125_CHECKSUM_BROKEN_PRE111.len(), 96);
        assert_eq!(M125_CHECKSUM_FIXED_SPEC111.len(), 96);
    }

    #[test]
    fn contract_checksum_drift_uses_shared_allow_helper() {
        let src = include_str!("m125.rs");
        assert!(
            src.contains("authorize_checksum_rewrite(MIGRATION_125_VERSION)")
                && src.contains("authorize_checksum_rewrite"),
            "LAW-MIG / X-02: m125 must use shared checksum_repair helper"
        );
    }
}
