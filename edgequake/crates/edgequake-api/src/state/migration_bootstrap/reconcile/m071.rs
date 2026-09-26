//! Migration 071 — HNSW dimension guard checksum repair (SPEC-042 / GitHub #275).
//!
//! v0.13.3 shipped M071 without pgvector HNSW dimension ceilings. Upgrades with
//! embeddings dim > 2000 (e.g. text-embedding-3-large @ 3072) failed at sqlx apply.
//! Fixed M071 promotes to halfvec when needed; repair checksum for already-applied rows.

use sqlx::PgPool;
use tracing::info;

use super::super::checksum_repair::authorize_checksum_rewrite;
use super::super::helpers::sqlx_migrations_table_exists;
use super::super::MIGRATION_071_VERSION;

/// SHA-384 of pre-#275 M071 (no dimension guard).
pub(super) const M071_CHECKSUM_PRE_275: &str =
    "fa6cce9c4b088b5dbc850764887e9bd119f32ceca259611113ab4673b6c3319353ae2b44218a3d312d03e3509f5520ca";

/// SHA-384 of #275-fixed M071 — must match `checksums.lock`.
pub(super) const M071_CHECKSUM_FIXED_275: &str =
    "fea7b113e1aab4f88d0c22a071ba78e043b94e38023450c8257c0a0027647193b5c0ca382a6009577e5f4f823a46cda2";

pub async fn repair_migration_071_checksum_if_needed(pool: &PgPool) -> Result<bool, sqlx::Error> {
    if !sqlx_migrations_table_exists(pool).await? {
        return Ok(false);
    }

    let current: Option<String> = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex') FROM _sqlx_migrations \
         WHERE version = $1 AND success = true",
    )
    .bind(MIGRATION_071_VERSION)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(false);
    };

    if current != M071_CHECKSUM_PRE_275 {
        return Ok(false);
    }

    // SPEC-150: known production fossils auto-accept; else scoped env.
    if let Err(msg) = authorize_checksum_rewrite(
        MIGRATION_071_VERSION,
        &current,
        "pre-#275 HNSW dimension guard",
    ) {
        return Err(sqlx::Error::Protocol(msg));
    }

    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
         WHERE version = $2 AND success = true",
    )
    .bind(M071_CHECKSUM_FIXED_275)
    .bind(MIGRATION_071_VERSION)
    .execute(pool)
    .await?;

    info!(
        target: "edgequake.migration",
        step = "migration_071_checksum_repair",
        from = M071_CHECKSUM_PRE_275,
        to = M071_CHECKSUM_FIXED_275,
        "Repaired migration 071 checksum (SPEC-042 #275 HNSW dimension guard; fossil/allow)"
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_and_fixed_checksums_are_distinct() {
        assert_ne!(M071_CHECKSUM_PRE_275, M071_CHECKSUM_FIXED_275);
        assert_eq!(M071_CHECKSUM_PRE_275.len(), 96);
        assert_eq!(M071_CHECKSUM_FIXED_275.len(), 96);
    }

    #[test]
    fn contract_checksum_drift_uses_shared_allow_helper() {
        let src = include_str!("m071.rs");
        assert!(
            src.contains("authorize_checksum_rewrite(MIGRATION_071_VERSION)")
                && src.contains("authorize_checksum_rewrite"),
            "LAW-MIG / X-02: M071 must use shared checksum_repair helper"
        );
    }
}
