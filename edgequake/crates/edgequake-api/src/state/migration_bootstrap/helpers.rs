//! Shared migration bootstrap helpers.

use sqlx::PgPool;

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

/// Return true if pgvector `extversion` is >= 0.8.0 (iterative-scan GUCs).
pub fn pgvector_supports_iterative_scan(version: &str) -> bool {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok());
    let major = parts.next();
    let minor = parts.next().unwrap_or(0);
    match major {
        Some(0) => minor >= 8,
        Some(_) => true,
        None => false,
    }
}
