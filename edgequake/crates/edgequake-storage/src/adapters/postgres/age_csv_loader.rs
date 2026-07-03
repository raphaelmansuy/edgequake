//! AGE CSV bulk loader (SPEC-042-E E-04) — uses load_labels_from_file (AGE 1.7+ COPY backend).

use sqlx::PgPool;

use crate::error::{Result, StorageError};

use super::capabilities::{age_supports_copy_loader, PostgresCapabilities};

/// Load vertices from a server-local CSV path via AGE `load_labels_from_file`.
pub async fn load_vertices_from_csv(
    pool: &PgPool,
    caps: &PostgresCapabilities,
    graph_name: &str,
    label_name: &str,
    csv_path: &str,
    row_count: usize,
) -> Result<u64> {
    if row_count < caps.age_copy_min_rows {
        return Err(StorageError::Database(format!(
            "Row count {} below COPY threshold {}",
            row_count, caps.age_copy_min_rows
        )));
    }
    if !age_supports_copy_loader(caps.age_extversion.as_deref()) {
        return Err(StorageError::Database(
            "AGE COPY loader requires AGE >= 1.7.0".into(),
        ));
    }

    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| StorageError::Connection(format!("Failed to acquire connection: {e}")))?;

    sqlx::query("LOAD 'age'")
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("LOAD age failed: {e}")))?;
    sqlx::query("SET search_path = ag_catalog, \"$user\", public")
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("search_path failed: {e}")))?;

    let sql = format!(
        "SELECT load_labels_from_file('{}', '{}', '{}')",
        graph_name.replace('\'', "''"),
        label_name.replace('\'', "''"),
        csv_path.replace('\'', "''")
    );

    let rows: Option<i64> = sqlx::query_scalar(&sql)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("load_labels_from_file failed: {e}")))?;

    Ok(rows.unwrap_or(row_count as i64) as u64)
}

/// Whether the COPY fast path should be used for the given row count.
pub fn should_use_copy_loader(caps: &PostgresCapabilities, row_count: usize) -> bool {
    row_count >= caps.age_copy_min_rows && caps.age_copy_loader_effective
}
