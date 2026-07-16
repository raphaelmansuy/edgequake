//! Shared migration bootstrap helpers.

use sqlx::PgPool;

/// Whether `public._sqlx_migrations` exists (fresh DBs have no tracking table yet).
pub(super) async fn sqlx_migrations_table_exists(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = '_sqlx_migrations'
        )",
    )
    .fetch_one(pool)
    .await
}

pub fn large_graph_threshold() -> i64 {
    std::env::var("EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500_000)
}

pub(super) async fn set_large_graph_threshold(
    pool: &PgPool,
    threshold: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('edgequake.migration_large_graph_threshold', $1, false)")
        .bind(threshold.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) fn quote_schema(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Compare dotted semver-like extension versions (e.g. `0.8.3`, `1.6.0`).
pub fn extension_version_at_least(version: &str, minimum: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    let va = parse(version);
    let vb = parse(minimum);
    if va.is_empty() {
        return false;
    }
    for i in 0..vb.len().max(va.len()) {
        let a = va.get(i).copied().unwrap_or(0);
        let b = vb.get(i).copied().unwrap_or(0);
        if a > b {
            return true;
        }
        if a < b {
            return false;
        }
    }
    true
}

/// Return true if pgvector `extversion` is >= 0.8.0 (iterative-scan GUCs).
pub fn pgvector_supports_iterative_scan(version: &str) -> bool {
    extension_version_at_least(version, "0.8.0")
}

/// CVE-2026-3172 floor — prefer ≥0.8.5 in images; block readiness below 0.8.2.
pub fn pgvector_meets_cve_floor(version: &str) -> bool {
    extension_version_at_least(version, "0.8.2")
}
