//! Maintained row counters for O(1) `count()` on large PostgreSQL tables.
//!
//! SPEC-011 / SPEC-012: stats table + INSERT/DELETE triggers.
//!
//! ## search_path safety
//!
//! DDL must run on a **single connection** with `SET LOCAL search_path TO public`
//! and **schema-qualified** function names. Otherwise `CREATE FUNCTION` may land in
//! `ag_catalog` (when a pooled connection still has graph `search_path`) while
//! `CREATE TRIGGER` on another connection with `search_path=public` fails with:
//! `function eq_*_stats_insert() does not exist`.

use sqlx::{Acquire, PgPool};

use crate::error::{Result, StorageError};

/// Configuration for maintained row-count stats (KV or vectors).
pub struct RowCountStatsConfig<'a> {
    pub prefix: &'a str,
    pub table_name: &'a str,
    pub stats_table_name: &'a str,
    /// `"kv"` or `"vectors"` — used in function/trigger name suffixes.
    pub kind: &'a str,
}

impl<'a> RowCountStatsConfig<'a> {
    fn fn_insert(&self) -> String {
        format!("eq_{}_{}_stats_insert", self.prefix, self.kind)
    }

    fn fn_delete(&self) -> String {
        format!("eq_{}_{}_stats_delete", self.prefix, self.kind)
    }

    fn trigger_insert(&self) -> String {
        format!("eq_{}_{}_stats_insert_trg", self.prefix, self.kind)
    }

    fn trigger_delete(&self) -> String {
        format!("eq_{}_{}_stats_delete_trg", self.prefix, self.kind)
    }
}

/// Create or refresh stats table, plpgsql functions, and triggers (idempotent).
pub async fn ensure_row_count_stats(pool: &PgPool, config: &RowCountStatsConfig<'_>) -> Result<()> {
    let fn_insert = config.fn_insert();
    let fn_delete = config.fn_delete();
    let trigger_insert = config.trigger_insert();
    let trigger_delete = config.trigger_delete();

    let mut conn = pool.acquire().await.map_err(|e| {
        StorageError::Database(format!("Failed to acquire connection for row stats: {}", e))
    })?;

    let mut tx = conn.begin().await.map_err(|e| {
        StorageError::Database(format!("Failed to begin row stats transaction: {}", e))
    })?;

    sqlx::query("SET LOCAL search_path TO public")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to set search_path for row stats: {}", e))
        })?;

    // Remove legacy copies accidentally created in ag_catalog (graph search_path pollution).
    let drop_ag_fn_insert = format!("DROP FUNCTION IF EXISTS ag_catalog.{}() CASCADE", fn_insert);
    let drop_ag_fn_delete = format!("DROP FUNCTION IF EXISTS ag_catalog.{}() CASCADE", fn_delete);
    sqlx::query(&drop_ag_fn_insert).execute(&mut *tx).await.ok();
    sqlx::query(&drop_ag_fn_delete).execute(&mut *tx).await.ok();

    let stats_sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {} (
            id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
            row_count BIGINT NOT NULL DEFAULT 0
        )
        "#,
        config.stats_table_name
    );
    sqlx::query(&stats_sql)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to create {} stats table: {}",
                config.kind, e
            ))
        })?;

    let backfill_sql = format!(
        r#"
        INSERT INTO {} (id, row_count)
        SELECT 1, COUNT(*)::bigint FROM {}
        ON CONFLICT (id) DO NOTHING
        "#,
        config.stats_table_name, config.table_name
    );
    sqlx::query(&backfill_sql).execute(&mut *tx).await.ok();

    let create_insert_fn = format!(
        r#"
        CREATE OR REPLACE FUNCTION public.{fn_insert}() RETURNS trigger AS $$
        BEGIN
            UPDATE {stats}
            SET row_count = row_count + 1
            WHERE id = 1;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#,
        fn_insert = fn_insert,
        stats = config.stats_table_name
    );
    sqlx::query(&create_insert_fn)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to create {} stats insert fn: {}",
                config.kind, e
            ))
        })?;

    let create_delete_fn = format!(
        r#"
        CREATE OR REPLACE FUNCTION public.{fn_delete}() RETURNS trigger AS $$
        BEGIN
            UPDATE {stats}
            SET row_count = GREATEST(row_count - 1, 0)
            WHERE id = 1;
            RETURN OLD;
        END;
        $$ LANGUAGE plpgsql
        "#,
        fn_delete = fn_delete,
        stats = config.stats_table_name
    );
    sqlx::query(&create_delete_fn)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to create {} stats delete fn: {}",
                config.kind, e
            ))
        })?;

    let drop_insert_trg = format!(
        "DROP TRIGGER IF EXISTS {trigger_insert} ON {table}",
        trigger_insert = trigger_insert,
        table = config.table_name
    );
    sqlx::query(&drop_insert_trg).execute(&mut *tx).await.ok();

    let create_insert_trg = format!(
        r#"
        CREATE TRIGGER {trigger_insert}
        AFTER INSERT ON {table}
        FOR EACH ROW EXECUTE FUNCTION public.{fn_insert}()
        "#,
        trigger_insert = trigger_insert,
        table = config.table_name,
        fn_insert = fn_insert
    );
    sqlx::query(&create_insert_trg)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to create {} stats insert trigger: {}",
                config.kind, e
            ))
        })?;

    let drop_delete_trg = format!(
        "DROP TRIGGER IF EXISTS {trigger_delete} ON {table}",
        trigger_delete = trigger_delete,
        table = config.table_name
    );
    sqlx::query(&drop_delete_trg).execute(&mut *tx).await.ok();

    let create_delete_trg = format!(
        r#"
        CREATE TRIGGER {trigger_delete}
        AFTER DELETE ON {table}
        FOR EACH ROW EXECUTE FUNCTION public.{fn_delete}()
        "#,
        trigger_delete = trigger_delete,
        table = config.table_name,
        fn_delete = fn_delete
    );
    sqlx::query(&create_delete_trg)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to create {} stats delete trigger: {}",
                config.kind, e
            ))
        })?;

    tx.commit().await.map_err(|e| {
        StorageError::Database(format!(
            "Failed to commit {} row stats setup: {}",
            config.kind, e
        ))
    })?;

    Ok(())
}
