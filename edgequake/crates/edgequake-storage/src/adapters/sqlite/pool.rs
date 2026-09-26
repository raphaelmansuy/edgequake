//! SQLite pool construction and deployment safety checks.

use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

use crate::StorageError;

const MIGRATION: &str = include_str!("../../../migrations_sqlite/001_provider_access.sql");
const MIGRATION_002: &str = include_str!("../../../migrations_sqlite/002_graph_contributions.sql");
const MIGRATION_003: &str =
    include_str!("../../../migrations_sqlite/003_projection_event_manifests.sql");

pub async fn connect_sqlite(path: impl AsRef<str>) -> Result<SqlitePool, StorageError> {
    validate_sqlite_deployment(path.as_ref())?;
    let path = path.as_ref();
    let in_memory = path == ":memory:" || path == "sqlite::memory:";
    let mut options = if in_memory {
        SqliteConnectOptions::new().in_memory(true)
    } else {
        SqliteConnectOptions::new()
            .filename(Path::new(path))
            .create_if_missing(true)
    };
    options = options
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .synchronous(SqliteSynchronous::Normal);
    if !in_memory {
        options = options.journal_mode(SqliteJournalMode::Wal);
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(if in_memory { 1 } else { 8 })
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(sqlite_error)?;
    sqlx::raw_sql(MIGRATION)
        .execute(&pool)
        .await
        .map_err(sqlite_error)?;
    sqlx::raw_sql(MIGRATION_002)
        .execute(&pool)
        .await
        .map_err(sqlite_error)?;
    sqlx::raw_sql(MIGRATION_003)
        .execute(&pool)
        .await
        .map_err(sqlite_error)?;
    Ok(pool)
}

pub fn validate_sqlite_deployment(path: &str) -> Result<(), StorageError> {
    let replicas = std::env::var("EDGEQUAKE_SQLITE_REPLICAS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    if replicas > 1 {
        return Err(StorageError::InvalidConfig(format!(
            "SQLite relational authority is single-replica; configured replicas={replicas}"
        )));
    }
    let topology = std::env::var("EDGEQUAKE_SQLITE_TOPOLOGY")
        .unwrap_or_else(|_| "single".into())
        .to_ascii_lowercase();
    if !matches!(topology.trim(), "single" | "standalone" | "local") {
        return Err(StorageError::InvalidConfig(format!(
            "SQLite topology must be single/standalone/local, got '{topology}'"
        )));
    }

    let normalized = path.to_ascii_lowercase();
    if normalized.starts_with("//")
        || normalized.starts_with("nfs://")
        || normalized.starts_with("smb://")
        || normalized.contains("/nfs/")
        || normalized.contains("/net/")
    {
        return Err(StorageError::InvalidConfig(format!(
            "SQLite authority path appears network-mounted and is unsupported: {path}"
        )));
    }
    Ok(())
}

fn sqlite_error(error: sqlx::Error) -> StorageError {
    StorageError::Database(format!("SQLite: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_paths_fail_closed() {
        assert!(validate_sqlite_deployment("//server/share/edgequake.db").is_err());
        assert!(validate_sqlite_deployment("/net/edgequake.db").is_err());
    }
}
