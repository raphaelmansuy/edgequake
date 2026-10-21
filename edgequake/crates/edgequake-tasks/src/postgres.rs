//! PostgreSQL task storage implementation.

#[cfg(feature = "postgres")]
use crate::config::{task_max_workers_from_env, CLAIM_SAMPLE_LIMIT};
#[cfg(feature = "postgres")]
use crate::fairness_hold::{lifecycle_task_type_sql, ClaimFairnessPolicy};
#[cfg(feature = "postgres")]
use crate::{
    error::{TaskError, TaskResult},
    storage::*,
    types::Task,
};
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use sqlx::{postgres::PgRow, Acquire, PgPool, Row};
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use std::time::Duration;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
const TASK_SELECT_COLUMNS: &str = r#"
    track_id, tenant_id, workspace_id, task_type, status, created_at, updated_at,
    started_at, completed_at, error_message, error, retry_count,
    max_retries, consecutive_timeout_failures, circuit_breaker_tripped,
    payload, progress, result, lease_owner, lease_token, lease_expires_at, pdf_id,
    fairness_hold_until
"#;

/// RETURNING list for `UPDATE tasks t … FROM candidate` — must qualify as `t.*`
/// because `candidate.track_id` makes bare `track_id` ambiguous (Postgres).
#[cfg(feature = "postgres")]
const TASK_RETURNING_COLUMNS_ALIASED: &str = r#"
    t.track_id, t.tenant_id, t.workspace_id, t.task_type, t.status, t.created_at, t.updated_at,
    t.started_at, t.completed_at, t.error_message, t.error, t.retry_count,
    t.max_retries, t.consecutive_timeout_failures, t.circuit_breaker_tripped,
    t.payload, t.progress, t.result, t.lease_owner, t.lease_token, t.lease_expires_at, t.pdf_id,
    t.fairness_hold_until
"#;

#[cfg(feature = "postgres")]
const LIST_STATEMENT_TIMEOUT_MS: u32 = 500;

#[cfg(feature = "postgres")]
fn pdf_id_column_value(task: &Task) -> Option<String> {
    task.pdf_id().map(|id| id.to_string())
}

#[cfg(feature = "postgres")]
fn task_from_row(row: &PgRow) -> TaskResult<Task> {
    let payload: serde_json::Value = row.get("payload");
    let task_data = payload
        .get("task_data")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let metadata =
        payload
            .get("metadata")
            .cloned()
            .and_then(|v| if v.is_null() { None } else { Some(v) });
    let progress = row
        .get::<Option<serde_json::Value>, _>("progress")
        .and_then(|v| if v.is_null() { None } else { Some(v) })
        .or_else(|| payload.get("progress").cloned())
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v).ok()
            }
        });

    Ok(Task {
        track_id: row.get("track_id"),
        tenant_id: row.get("tenant_id"),
        workspace_id: row.get("workspace_id"),
        task_type: row
            .get::<String, _>("task_type")
            .parse()
            .map_err(|_| TaskError::InvalidTaskData("Invalid task type".to_string()))?,
        status: row
            .get::<String, _>("status")
            .parse()
            .map_err(|_| TaskError::InvalidTaskData("Invalid status".to_string()))?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        error_message: row.get("error_message"),
        error: row
            .get::<Option<serde_json::Value>, _>("error")
            .and_then(|v| serde_json::from_value(v).ok()),
        retry_count: row.get("retry_count"),
        max_retries: row.get("max_retries"),
        consecutive_timeout_failures: row.get("consecutive_timeout_failures"),
        circuit_breaker_tripped: row.get("circuit_breaker_tripped"),
        task_data,
        metadata,
        progress,
        result: row.get("result"),
        lease_owner: row.get("lease_owner"),
        lease_token: row.get("lease_token"),
        lease_expires_at: row.get("lease_expires_at"),
        fairness_hold_until: row.try_get("fairness_hold_until").ok().flatten(),
    })
}

#[cfg(feature = "postgres")]
/// PostgreSQL task storage
#[derive(Debug, Clone)]
pub struct PostgresTaskStorage {
    pool: Arc<PgPool>,
    /// Cached `rolsuper OR rolbypassrls`. Superuser sessions keep every
    /// statement on the historical pool path.
    bypasses_rls: Arc<std::sync::OnceLock<bool>>,
}

#[cfg(feature = "postgres")]
impl PostgresTaskStorage {
    /// Create a new PostgreSQL storage
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
            bypasses_rls: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Create from an Arc pool
    pub fn from_arc(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            bypasses_rls: Arc::new(std::sync::OnceLock::new()),
        }
    }

    async fn rls_bypassed(&self) -> TaskResult<bool> {
        if let Some(bypass) = self.bypasses_rls.get() {
            return Ok(*bypass);
        }
        let bypass = crate::task_rls::pool_bypasses_rls(&self.pool).await?;
        let _ = self.bypasses_rls.set(bypass);
        Ok(self.bypasses_rls.get().copied().unwrap_or(bypass))
    }

    async fn touch_task_scoped(&self, track_id: &str) -> TaskResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to begin touch_task transaction: {e}"))
        })?;
        let tenants = crate::task_rls::active_tenants(&mut tx).await?;
        for tenant in tenants {
            crate::task_rls::install_tenant_scope(&mut tx, tenant, None).await?;
            let affected = sqlx::query("UPDATE tasks SET updated_at = NOW() WHERE track_id = $1")
                .bind(track_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| TaskError::StorageError(format!("Failed to touch task: {e}")))?
                .rows_affected();
            if affected > 0 {
                tx.commit().await.map_err(|e| {
                    TaskError::StorageError(format!("Failed to commit touch_task: {e}"))
                })?;
                return Ok(());
            }
        }
        let _ = tx.rollback().await;
        Ok(())
    }

    async fn fetch_task_in_workspace(
        &self,
        sql: &str,
        workspace_id: Uuid,
        pdf_id: &str,
    ) -> TaskResult<Option<Task>> {
        let tenant: Option<Uuid> = sqlx::query_scalar(
            "SELECT tenant_id FROM public.workspaces WHERE workspace_id = $1 AND is_active",
        )
        .bind(workspace_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to resolve workspace tenant: {e}")))?;
        let Some(tenant) = tenant else {
            return Ok(None);
        };
        let mut tx = self.pool.begin().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to begin workspace task lookup: {e}"))
        })?;
        crate::task_rls::install_tenant_scope(&mut tx, tenant, Some(workspace_id)).await?;
        let row = sqlx::query(sql)
            .bind(workspace_id)
            .bind(pdf_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to fetch workspace task: {e}")))?;
        tx.commit().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to commit workspace task lookup: {e}"))
        })?;
        match row {
            Some(row) => Ok(Some(task_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn update_progress_scoped(
        &self,
        track_id: &str,
        progress_json: &serde_json::Value,
    ) -> TaskResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to begin progress transaction: {e}"))
        })?;
        let tenants = crate::task_rls::active_tenants(&mut tx).await?;
        for tenant in tenants {
            crate::task_rls::install_tenant_scope(&mut tx, tenant, None).await?;
            let affected = sqlx::query(
                "UPDATE tasks SET progress = $2, updated_at = NOW() WHERE track_id = $1",
            )
            .bind(track_id)
            .bind(progress_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to update task progress: {e}")))?
            .rows_affected();
            if affected > 0 {
                break;
            }
        }
        tx.commit().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to commit progress transaction: {e}"))
        })?;
        Ok(())
    }
}

/// `$1 IS NULL OR tenant_id = $1` hides the tenant from a generic plan, so
/// queue metrics seq-scans every task. When the caller passes a tenant or a
/// workspace, the predicate is required and `idx_tasks_tenant_workspace_status`
/// can start at those rows. An unscoped call keeps the open predicate.
#[cfg(feature = "postgres")]
fn queue_metrics_statement(has_tenant: bool, has_workspace: bool) -> String {
    const HEAD: &str = r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
                COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
                CAST(COALESCE(AVG(EXTRACT(EPOCH FROM (started_at - created_at)))
                    FILTER (WHERE started_at IS NOT NULL), 0) AS DOUBLE PRECISION) as avg_wait_seconds,
                CAST(COALESCE(MAX(EXTRACT(EPOCH FROM (NOW() - created_at)))
                    FILTER (WHERE status = 'pending'), 0) AS DOUBLE PRECISION) as max_wait_seconds,
                COUNT(*) FILTER (
                    WHERE status = 'indexed'
                    AND completed_at > NOW() - INTERVAL '5 minutes'
                ) as recent_completed
            FROM tasks
            "#;
    let predicate = match (has_tenant, has_workspace) {
        (true, true) => "WHERE tenant_id = $1 AND workspace_id = $2",
        (true, false) => "WHERE tenant_id = $1",
        (false, true) => "WHERE workspace_id = $1",
        (false, false) => {
            "WHERE ($1::uuid IS NULL OR tenant_id = $1) AND ($2::uuid IS NULL OR workspace_id = $2)"
        }
    };
    format!("{HEAD}{predicate}")
}

/// Oldest runnable pending tasks. `fairness_hold_until IS NULL OR hold <= now()`
/// hides the hold from a generic plan, so the sample walks older held rows.
/// Each arm requires one predicate. `idx_tasks_pending_unheld_created` serves
/// the null arm. `idx_tasks_fairness_hold_until` serves the due arm.
#[cfg(feature = "postgres")]
pub const FAIR_PENDING_SAMPLE_SQL: &str = r#"
SELECT track_id, tenant_id, workspace_id, task_type, created_at FROM (
    (SELECT track_id, tenant_id, workspace_id, task_type, created_at
     FROM tasks
     WHERE status = 'pending'
       AND fairness_parked_at IS NULL
       AND fairness_hold_until IS NULL
     ORDER BY created_at ASC
     LIMIT $1)
    UNION ALL
    (SELECT track_id, tenant_id, workspace_id, task_type, created_at
     FROM tasks
     WHERE status = 'pending'
       AND fairness_parked_at IS NULL
       AND fairness_hold_until <= NOW()
     ORDER BY created_at ASC
     LIMIT $1)
) pending_page
ORDER BY created_at ASC
LIMIT $1
"#;

/// Stale processing sample. A null lease and an expired lease are separate
/// arms, and each arm requires the hold predicate. `idx_tasks_processing_unleased_created`
/// serves a null lease with no hold. `idx_tasks_processing_expired_unheld`
/// serves an expired lease with no hold. A due hold uses `idx_tasks_fairness_hold_until`.
#[cfg(feature = "postgres")]
pub const FAIR_STALE_SAMPLE_SQL: &str = r#"
SELECT track_id, tenant_id, workspace_id, task_type, created_at FROM (
    (SELECT track_id, tenant_id, workspace_id, task_type, created_at
     FROM tasks
     WHERE status = 'processing'
       AND lease_expires_at IS NULL
       AND fairness_hold_until IS NULL
     ORDER BY created_at ASC
     LIMIT $1)
    UNION ALL
    (SELECT track_id, tenant_id, workspace_id, task_type, created_at
     FROM tasks
     WHERE status = 'processing'
       AND lease_expires_at IS NULL
       AND fairness_hold_until <= NOW()
     ORDER BY created_at ASC
     LIMIT $1)
    UNION ALL
    (SELECT track_id, tenant_id, workspace_id, task_type, created_at
     FROM tasks
     WHERE status = 'processing'
       AND lease_expires_at < NOW()
       AND fairness_hold_until IS NULL
     ORDER BY created_at ASC
     LIMIT $1)
    UNION ALL
    (SELECT track_id, tenant_id, workspace_id, task_type, created_at
     FROM tasks
     WHERE status = 'processing'
       AND lease_expires_at < NOW()
       AND fairness_hold_until <= NOW()
     ORDER BY created_at ASC
     LIMIT $1)
) stale_page
ORDER BY created_at ASC
LIMIT $1
"#;

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl TaskStorage for PostgresTaskStorage {
    async fn create_task(&self, task: &Task) -> TaskResult<()> {
        // SPEC-090 F-090-13: ensure next monthly partitions exist (no-op if unpartitioned).
        let _ = sqlx::query("SELECT edgequake_ensure_tasks_month_partitions()")
            .execute(&*self.pool)
            .await;

        // SPEC-090 F-090-04: progress lives only in the column — never in payload.
        let payload = serde_json::json!({
            "task_data": task.task_data,
            "metadata": task.metadata,
        });
        let progress_json = task
            .progress
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| TaskError::StorageError(format!("Failed to serialize progress: {e}")))?;
        let pdf_id = pdf_id_column_value(task);
        let document_id = task.document_id();

        let mut scoped = None;
        if !self.rls_bypassed().await? {
            let mut tx = self.pool.begin().await.map_err(|e| {
                TaskError::StorageError(format!("Failed to begin create_task transaction: {e}"))
            })?;
            crate::task_rls::install_tenant_scope(&mut tx, task.tenant_id, Some(task.workspace_id))
                .await?;
            scoped = Some(tx);
        }

        let query = sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, tenant_id, workspace_id, task_type, status, created_at, updated_at,
                started_at, completed_at, error_message, error, retry_count,
                max_retries, consecutive_timeout_failures, circuit_breaker_tripped,
                payload, progress, result, lease_owner, lease_token, lease_expires_at, pdf_id,
                document_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
            "#,
        )
        .bind(&task.track_id)
        .bind(task.tenant_id)
        .bind(task.workspace_id)
        .bind(task.task_type.to_string())
        .bind(task.status.to_string())
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(&task.error_message)
        .bind(serde_json::to_value(&task.error)?)
        .bind(task.retry_count)
        .bind(task.max_retries)
        .bind(task.consecutive_timeout_failures)
        .bind(task.circuit_breaker_tripped)
        .bind(&payload)
        .bind(&progress_json)
        .bind(&task.result)
        .bind(&task.lease_owner)
        .bind(task.lease_token)
        .bind(task.lease_expires_at)
        .bind(&pdf_id)
        .bind(&document_id);
        let result = if let Some(tx) = scoped.as_mut() {
            query.execute(&mut **tx).await
        } else {
            query.execute(&*self.pool).await
        };
        result.map_err(|e| TaskError::StorageError(format!("Failed to create task: {}", e)))?;
        if let Some(tx) = scoped {
            tx.commit().await.map_err(|e| {
                TaskError::StorageError(format!("Failed to commit create_task transaction: {e}"))
            })?;
        }

        Ok(())
    }

    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>> {
        if !self.rls_bypassed().await? {
            let mut tx = self.pool.begin().await.map_err(|e| {
                TaskError::StorageError(format!("Failed to begin get_task transaction: {e}"))
            })?;
            let tenants = crate::task_rls::active_tenants(&mut tx).await?;
            let sql = format!("SELECT {TASK_SELECT_COLUMNS} FROM tasks WHERE track_id = $1");
            for tenant in tenants {
                crate::task_rls::install_tenant_scope(&mut tx, tenant, None).await?;
                if let Some(row) = sqlx::query(&sql)
                    .bind(track_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| TaskError::StorageError(format!("Failed to fetch task: {e}")))?
                {
                    tx.commit().await.map_err(|e| {
                        TaskError::StorageError(format!("Failed to commit get_task: {e}"))
                    })?;
                    return Ok(Some(task_from_row(&row)?));
                }
            }
            let _ = tx.rollback().await;
            return Ok(None);
        }
        let sql = format!("SELECT {TASK_SELECT_COLUMNS} FROM tasks WHERE track_id = $1");
        let row = sqlx::query(&sql)
            .bind(track_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to fetch task: {}", e)))?;

        match row {
            Some(row) => Ok(Some(task_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_tasks_by_track_ids(
        &self,
        track_ids: &[String],
    ) -> TaskResult<std::collections::HashMap<String, Task>> {
        let mut out = std::collections::HashMap::with_capacity(track_ids.len());
        if track_ids.is_empty() {
            return Ok(out);
        }
        let sql = format!("SELECT {TASK_SELECT_COLUMNS} FROM tasks WHERE track_id = ANY($1)");
        let rows = sqlx::query(&sql)
            .bind(track_ids)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to batch-fetch tasks: {}", e)))?;
        for row in rows {
            let task = task_from_row(&row)?;
            out.insert(task.track_id.clone(), task);
        }
        Ok(out)
    }

    /// Lightweight heartbeat: only update `updated_at` column.
    ///
    /// WHY: This is ~10x cheaper than a full `update_task` because it doesn't
    /// serialize/deserialize the JSONB payload column. Workers call this every
    /// 60 seconds during long-running LLM extraction to signal liveness.
    async fn touch_task(&self, track_id: &str) -> TaskResult<()> {
        if !self.rls_bypassed().await? {
            return self.touch_task_scoped(track_id).await;
        }
        sqlx::query("UPDATE tasks SET updated_at = NOW() WHERE track_id = $1")
            .bind(track_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to touch task: {}", e)))?;
        Ok(())
    }

    async fn update_task_progress(
        &self,
        track_id: &str,
        progress: &crate::types::TaskProgress,
    ) -> TaskResult<()> {
        let progress_json = serde_json::to_value(progress)
            .map_err(|e| TaskError::StorageError(format!("Failed to serialize progress: {e}")))?;
        if !self.rls_bypassed().await? {
            return self.update_progress_scoped(track_id, &progress_json).await;
        }
        let result = sqlx::query(
            r#"
            UPDATE tasks SET progress = $2, updated_at = NOW()
            WHERE track_id = $1
            "#,
        )
        .bind(track_id)
        .bind(&progress_json)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to update task progress: {e}")))?;
        // 0 rows: lifecycle purge deleted the row mid-heartbeat — idempotent.
        let _ = result.rows_affected();
        Ok(())
    }

    async fn update_task(&self, task: &Task) -> TaskResult<()> {
        // SPEC-090 F-090-04: progress column is hot; payload omits progress on update.
        let payload = serde_json::json!({
            "task_data": task.task_data,
            "metadata": task.metadata,
        });
        let progress_json = task
            .progress
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| TaskError::StorageError(format!("Failed to serialize progress: {e}")))?;

        let mut scoped = None;
        if !self.rls_bypassed().await? {
            let mut tx = self.pool.begin().await.map_err(|e| {
                TaskError::StorageError(format!("Failed to begin update_task transaction: {e}"))
            })?;
            crate::task_rls::install_tenant_scope(&mut tx, task.tenant_id, Some(task.workspace_id))
                .await?;
            scoped = Some(tx);
        }

        let query = sqlx::query(
            r#"
            UPDATE tasks SET
                status = $2,
                updated_at = $3,
                started_at = $4,
                completed_at = $5,
                error_message = $6,
                error = $7,
                retry_count = $8,
                consecutive_timeout_failures = $9,
                circuit_breaker_tripped = $10,
                payload = $11,
                progress = $12,
                result = $13,
                lease_owner = $14,
                lease_token = $15,
                lease_expires_at = $16
            WHERE track_id = $1
            "#,
        )
        .bind(&task.track_id)
        .bind(task.status.to_string())
        .bind(task.updated_at)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(&task.error_message)
        .bind(serde_json::to_value(&task.error)?)
        .bind(task.retry_count)
        .bind(task.consecutive_timeout_failures)
        .bind(task.circuit_breaker_tripped)
        .bind(&payload)
        .bind(&progress_json)
        .bind(&task.result)
        .bind(&task.lease_owner)
        .bind(task.lease_token)
        .bind(task.lease_expires_at);
        let result = if let Some(tx) = scoped.as_mut() {
            query.execute(&mut **tx).await
        } else {
            query.execute(&*self.pool).await
        };
        let result =
            result.map_err(|e| TaskError::StorageError(format!("Failed to update task: {}", e)))?;
        if let Some(tx) = scoped {
            tx.commit().await.map_err(|e| {
                TaskError::StorageError(format!("Failed to commit update_task transaction: {e}"))
            })?;
        }

        if result.rows_affected() == 0 {
            return Err(TaskError::TaskNotFound(task.track_id.clone()));
        }

        Ok(())
    }

    async fn delete_task(&self, track_id: &str) -> TaskResult<()> {
        let result = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(track_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to delete task: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(TaskError::TaskNotFound(track_id.to_string()));
        }

        Ok(())
    }

    async fn list_tasks(&self, filter: TaskFilter, pagination: Pagination) -> TaskResult<TaskList> {
        let mut query = format!("SELECT {TASK_SELECT_COLUMNS} FROM tasks WHERE 1=1");

        let mut param_count = 0;

        if filter.tenant_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND tenant_id = ${}", param_count));
        }
        if filter.workspace_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND workspace_id = ${}", param_count));
        }

        if filter.status.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND status = ${}", param_count));
        }
        if filter.task_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND task_type = ${}", param_count));
        }

        let keyset_created_at = pagination.after_created_at;
        let keyset_track_id = pagination.after_track_id.as_deref().map(str::to_owned);
        let use_keyset = pagination.has_keyset_cursor()
            && pagination.sort_by == SortField::CreatedAt
            && keyset_created_at.is_some()
            && keyset_track_id.is_some();

        if use_keyset {
            param_count += 1;
            let created_param = param_count;
            param_count += 1;
            let track_param = param_count;
            match pagination.order {
                SortOrder::Desc => query.push_str(&format!(
                    " AND (created_at, track_id) < (${created_param}, ${track_param})"
                )),
                SortOrder::Asc => query.push_str(&format!(
                    " AND (created_at, track_id) > (${created_param}, ${track_param})"
                )),
            }
        }

        let sort_field = match pagination.sort_by {
            SortField::CreatedAt => "created_at",
            SortField::UpdatedAt => "updated_at",
        };
        let sort_order = match pagination.order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        if pagination.sort_by == SortField::CreatedAt {
            query.push_str(&format!(
                " ORDER BY {sort_field} {sort_order}, track_id {sort_order}"
            ));
        } else {
            query.push_str(&format!(" ORDER BY {sort_field} {sort_order}"));
        }

        param_count += 1;
        let limit_param = param_count;
        query.push_str(&format!(" LIMIT ${limit_param}"));

        let offset = if use_keyset || pagination.page <= 1 {
            0
        } else {
            (pagination.page - 1) * pagination.page_size
        };
        if offset > 0 {
            param_count += 1;
            let offset_param = param_count;
            query.push_str(&format!(" OFFSET ${offset_param}"));
        }

        let mut conn = self.pool.acquire().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to acquire connection for list_tasks: {e}"))
        })?;
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to begin list_tasks txn: {e}")))?;
        if !crate::task_rls::session_bypasses_rls(&mut tx).await? {
            if let Some(tenant) = filter.tenant_id {
                crate::task_rls::install_tenant_scope(&mut tx, tenant, filter.workspace_id).await?;
            }
        }
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{LIST_STATEMENT_TIMEOUT_MS}ms'"
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!("Failed to set list_tasks statement_timeout: {e}"))
        })?;

        let mut query_builder = sqlx::query(&query);

        if let Some(tenant_id) = &filter.tenant_id {
            query_builder = query_builder.bind(tenant_id);
        }
        if let Some(workspace_id) = &filter.workspace_id {
            query_builder = query_builder.bind(workspace_id);
        }
        if let Some(status) = &filter.status {
            query_builder = query_builder.bind(status.to_string());
        }
        if let Some(task_type) = &filter.task_type {
            query_builder = query_builder.bind(task_type.to_string());
        }
        if use_keyset {
            query_builder = query_builder
                .bind(keyset_created_at.unwrap())
                .bind(keyset_track_id.as_deref().unwrap());
        }
        query_builder = query_builder.bind(i64::from(pagination.page_size));
        if offset > 0 {
            query_builder = query_builder.bind(i64::from(offset));
        }

        let rows = match query_builder.fetch_all(&mut *tx).await {
            Ok(rows) => rows,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(TaskError::StorageError(format!(
                    "Failed to list tasks: {e}"
                )));
            }
        };

        tx.commit().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to commit list_tasks txn: {e}"))
        })?;

        let tasks: Vec<Task> = rows
            .into_iter()
            .filter_map(|row| task_from_row(&row).ok())
            .collect();

        let total = self.get_estimated_count(filter).await?;
        let total_pages = ((total as f64) / (pagination.page_size as f64)).ceil() as u32;

        Ok(TaskList {
            tasks,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages,
        })
    }

    async fn get_statistics(&self, filter: TaskFilter) -> TaskResult<TaskStatistics> {
        // WHY: Build dynamic SQL to support tenant/workspace filtering
        // Without this, statistics would leak across tenant boundaries.
        //
        // SPEC-089 Wave 3 / F-336-09 / LAW-H2: health & list skip wrap this in
        // tokio::timeout (750ms / 550ms). SET LOCAL 500ms kills COUNTs before
        // the app abandons so they cannot zombie the shared pool (GH-336 class).
        const STATS_STATEMENT_TIMEOUT_MS: u32 = 500;

        let mut query = String::from(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'processing') as processing,
                COUNT(*) FILTER (WHERE status = 'indexed') as indexed,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled,
                COUNT(*) as total
            FROM tasks
            WHERE 1=1
            "#,
        );

        let mut param_count = 0;

        // Add tenant filter if present
        if filter.tenant_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND tenant_id = ${}", param_count));
        }

        // Add workspace filter if present
        if filter.workspace_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND workspace_id = ${}", param_count));
        }

        // Add status filter if present
        if filter.status.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND status = ${}", param_count));
        }

        // Add task_type filter if present
        if filter.task_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND task_type = ${}", param_count));
        }

        let mut conn = self.pool.acquire().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to acquire connection for statistics: {e}"))
        })?;
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to begin statistics txn: {e}")))?;
        if !crate::task_rls::session_bypasses_rls(&mut tx).await? {
            if let Some(tenant) = filter.tenant_id {
                crate::task_rls::install_tenant_scope(&mut tx, tenant, filter.workspace_id).await?;
            }
        }
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{STATS_STATEMENT_TIMEOUT_MS}ms'"
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!("Failed to set statistics statement_timeout: {e}"))
        })?;

        // Build and bind query
        let mut sqlx_query = sqlx::query(&query);

        if let Some(tenant_id) = filter.tenant_id {
            sqlx_query = sqlx_query.bind(tenant_id);
        }

        if let Some(workspace_id) = filter.workspace_id {
            sqlx_query = sqlx_query.bind(workspace_id);
        }

        if let Some(status) = filter.status {
            sqlx_query = sqlx_query.bind(status.to_string());
        }

        if let Some(task_type) = filter.task_type {
            sqlx_query = sqlx_query.bind(task_type.to_string());
        }

        let row = match sqlx_query.fetch_one(&mut *tx).await {
            Ok(row) => {
                tx.commit().await.map_err(|e| {
                    TaskError::StorageError(format!("Failed to commit statistics txn: {e}"))
                })?;
                row
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(TaskError::StorageError(format!(
                    "Failed to get statistics: {e}"
                )));
            }
        };

        Ok(TaskStatistics {
            pending: row.get::<i64, _>("pending") as u64,
            processing: row.get::<i64, _>("processing") as u64,
            indexed: row.get::<i64, _>("indexed") as u64,
            failed: row.get::<i64, _>("failed") as u64,
            cancelled: row.get::<i64, _>("cancelled") as u64,
            total: row.get::<i64, _>("total") as u64,
        })
    }

    async fn find_active_pdf_processing_task(
        &self,
        pdf_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> TaskResult<Option<Task>> {
        let pdf_id_str = pdf_id.to_string();
        let sql = format!(
            r#"
            SELECT {TASK_SELECT_COLUMNS}
            FROM tasks
            WHERE workspace_id = $1
              AND pdf_id = $2
              AND status IN ('pending', 'processing')
              AND task_type IN ('pdf_processing', 'insert')
            ORDER BY created_at DESC
            LIMIT 1
            "#
        );
        if !self.rls_bypassed().await? {
            return self
                .fetch_task_in_workspace(&sql, workspace_id, &pdf_id_str)
                .await;
        }
        let row = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(&pdf_id_str)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                TaskError::StorageError(format!("Failed to find active PDF task: {}", e))
            })?;

        match row {
            Some(row) => Ok(Some(task_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_active_pdf_ingest_task(
        &self,
        pdf_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> TaskResult<Option<Task>> {
        let pdf_id_str = pdf_id.to_string();
        let sql = format!(
            r#"
            SELECT {TASK_SELECT_COLUMNS}
            FROM tasks
            WHERE workspace_id = $1
              AND pdf_id = $2
              AND task_type = 'insert'
              AND status IN ('pending', 'processing')
            ORDER BY created_at DESC
            LIMIT 1
            "#
        );
        if !self.rls_bypassed().await? {
            return self
                .fetch_task_in_workspace(&sql, workspace_id, &pdf_id_str)
                .await;
        }
        let row = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(&pdf_id_str)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                TaskError::StorageError(format!("Failed to find active PDF ingest task: {}", e))
            })?;

        match row {
            Some(row) => Ok(Some(task_from_row(&row)?)),
            None => Ok(None),
        }
    }

    // @dataop      DATA-PG-TASKS-CLAIM-NEXT-140
    // @engine      postgres
    // @intent      Hold-aware, tenant-priority, workspace-fair claim (SKIP LOCKED).
    // @tables      tasks
    // @indexes     idx_tasks_pending_unheld_created, idx_tasks_pending_unheld_workspace, idx_tasks_processing_unleased_created, idx_tasks_processing_unleased_workspace, idx_tasks_processing_expired_unheld, idx_tasks_processing_expired_unheld_workspace, idx_tasks_fairness_hold_until
    // @complexity  time: O(B + W) bounded sample B≈1000 + ws/tenant load; space: O(1)
    // @limits      Concurrent workers safe via FOR UPDATE SKIP LOCKED (two sargable arms). Lease TTL required; expired processing rows reclaimable. Active fairness_hold_until excludes Pending from claim (SPEC-057 INV-06). fairness_parked_at excluded via state machine CLAIM_PENDING_GUARD_SQL (SPEC-091 R-18). Deadlock risk if other paths lock tasks by track_id inconsistently.
    // @scaling     Cost bounded by sample size, not full backlog depth (SPEC-090 F-090-11)
    // @tests       tests/e2e_spec090_claim_bounded.rs, tests/postgres_claim_lease.rs
    // @pgversions  16: ok | 17: ok | 18: ok
    // @docs        specs/088-data-layer/postgres.md#data-pg-tasks-claim-next-140
    async fn claim_next_with_policy(
        &self,
        worker_id: &str,
        lease_ttl: Duration,
        policy: ClaimFairnessPolicy,
    ) -> TaskResult<Option<Task>> {
        let lease_token = Uuid::new_v4();
        let lease_expires_at = crate::lease_expires_at(Utc::now(), lease_ttl);
        let max_ingest = policy.max_ingest_per_tenant as i64;
        let max_lifecycle = policy.max_lifecycle_per_tenant as i64;

        let mut conn = self.pool.acquire().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to acquire connection for claim_next: {e}"))
        })?;
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to begin claim_next txn: {e}")))?;

        // Superuser claim is one pass over every visible row. A forced-RLS
        // login sees one tenant per pass; rotate the start so a busy tenant
        // does not hide the others.
        let subject_to_rls = !crate::task_rls::session_bypasses_rls(&mut tx).await?;
        let scope_tenants: Vec<Option<Uuid>> = if subject_to_rls {
            let mut ids = crate::task_rls::active_tenants(&mut tx).await?;
            if ids.is_empty() {
                let _ = tx.rollback().await;
                return Ok(None);
            }
            let start = (Utc::now().timestamp().rem_euclid(ids.len() as i64)) as usize;
            ids.rotate_left(start);
            ids.into_iter().map(Some).collect()
        } else {
            vec![None]
        };

        // SPEC-057 INV-06: exclude active holds; prefer under-cap tenants; then
        // SPEC-084 workspace-fair (least loaded, oldest). FP-2: held Pending and
        // active Processing both count toward tenant lane load; within a workspace
        // claim ORDER BY at_cap then created_at (parity with MemoryTaskStorage).
        let lifecycle = lifecycle_task_type_sql();
        let tenant_inflight_sql = format!(
            r#"
            SELECT
                tenant_id,
                COUNT(*) FILTER (
                    WHERE task_type IN ({lifecycle})
                )::bigint AS lifecycle_n,
                COUNT(*) FILTER (
                    WHERE task_type NOT IN ({lifecycle})
                )::bigint AS ingest_n
            FROM (
                SELECT tenant_id, task_type
                FROM tasks
                WHERE status = 'processing'
                  AND lease_expires_at IS NOT NULL
                  AND lease_expires_at >= NOW()
                UNION ALL
                SELECT tenant_id, task_type
                FROM tasks
                WHERE status = 'pending'
                  AND fairness_hold_until IS NOT NULL
                  AND fairness_hold_until > NOW()
            ) lane_load
            GROUP BY tenant_id
            "#,
            lifecycle = lifecycle
        );
        let at_cap_expr = format!(
            r#"
            CASE
                WHEN t2.task_type IN ({lifecycle}) THEN
                    CASE
                        WHEN {max_lifecycle} = 0 THEN 0
                        WHEN COALESCE(ti.lifecycle_n, 0) < {max_lifecycle} THEN 0
                        ELSE 1
                    END
                ELSE
                    CASE
                        WHEN {max_ingest} = 0 THEN 0
                        WHEN COALESCE(ti.ingest_n, 0) < {max_ingest} THEN 0
                        ELSE 1
                    END
            END
            "#,
            lifecycle = lifecycle,
            max_ingest = max_ingest,
            max_lifecycle = max_lifecycle
        );

        let fair_pick_sql = format!(
            r#"
            /* DATA-PG-TASKS-CLAIM-NEXT-140 — hold-aware + tenant-priority fair pick */
            WITH bounded_pending AS (
                {pending_sample}
            ),
            bounded_stale AS (
                {stale_sample}
            ),
            claimable AS (
                SELECT * FROM bounded_pending
                UNION ALL
                SELECT * FROM bounded_stale
            ),
            tenant_inflight AS (
                {tenant_inflight}
            ),
            ws_load AS (
                SELECT workspace_id, COUNT(*)::bigint AS active_count
                FROM tasks
                WHERE status = 'processing'
                  AND lease_expires_at IS NOT NULL
                  AND lease_expires_at >= NOW()
                GROUP BY workspace_id
            ),
            scored AS (
                SELECT
                    c.workspace_id,
                    c.created_at,
                    CASE
                        WHEN c.task_type IN ({lifecycle}) THEN
                            CASE
                                WHEN $3::bigint = 0 THEN 0
                                WHEN COALESCE(ti.lifecycle_n, 0) < $3::bigint THEN 0
                                ELSE 1
                            END
                        ELSE
                            CASE
                                WHEN $2::bigint = 0 THEN 0
                                WHEN COALESCE(ti.ingest_n, 0) < $2::bigint THEN 0
                                ELSE 1
                            END
                    END AS at_cap
                FROM claimable c
                LEFT JOIN tenant_inflight ti ON ti.tenant_id = c.tenant_id
            )
            SELECT s.workspace_id
            FROM scored s
            LEFT JOIN ws_load w ON w.workspace_id = s.workspace_id
            GROUP BY s.workspace_id
            ORDER BY
                MIN(s.at_cap) ASC,
                COALESCE(MAX(w.active_count), 0) ASC,
                MIN(s.created_at) ASC
            LIMIT 1
            "#,
            tenant_inflight = tenant_inflight_sql,
            lifecycle = lifecycle,
            pending_sample = FAIR_PENDING_SAMPLE_SQL,
            stale_sample = FAIR_STALE_SAMPLE_SQL,
        );

        for scope in scope_tenants {
            if let Some(tenant) = scope {
                crate::task_rls::install_tenant_scope(&mut tx, tenant, None).await?;
            }

            let fair_ws: Option<Uuid> = sqlx::query_scalar(&fair_pick_sql)
                .bind(CLAIM_SAMPLE_LIMIT)
                .bind(max_ingest)
                .bind(max_lifecycle)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| {
                    TaskError::StorageError(format!("Failed to pick fair workspace: {e}"))
                })?;

            let Some(fair_ws) = fair_ws else {
                continue;
            };

            // SPEC-091 QW0 (LAW-Q2): guard fragments come from the state machine SSOT
            // (`CLAIM_PENDING_GUARD_SQL`, `CLAIM_STALE_GUARD_SQL`). Each arm requires
            // one predicate so a generic plan can start at that index and stop at
            // one row. A single `IS NULL OR` walks every older held task.
            let claim_arm_sql = |arms: &[&str]| {
                let union = arms
                    .iter()
                    .map(|arm| {
                        format!(
                            r#"
                        (SELECT picked.track_id, picked.at_cap, picked.created_at
                         FROM (SELECT 1) AS seed
                         JOIN LATERAL (
                             SELECT t2.track_id, ({at_cap}) AS at_cap, t2.created_at
                             FROM tasks t2
                             LEFT JOIN (
                                 {tenant_inflight}
                             ) ti ON ti.tenant_id = t2.tenant_id
                             WHERE t2.workspace_id = $4
                               AND {arm}
                             ORDER BY ({at_cap}) ASC, t2.created_at ASC
                             FOR UPDATE OF t2 SKIP LOCKED
                             LIMIT 1
                         ) picked ON true)"#,
                            at_cap = at_cap_expr,
                            tenant_inflight = tenant_inflight_sql,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" UNION ALL ");
                format!(
                    r#"
                UPDATE tasks t
                SET status = 'processing',
                    lease_owner = $1,
                    lease_token = $2,
                    lease_expires_at = $3,
                    fairness_hold_until = NULL,
                    started_at = COALESCE(t.started_at, NOW()),
                    updated_at = NOW(),
                    completed_at = NULL
                FROM (
                    SELECT track_id FROM (
                        {union}
                    ) arms
                    ORDER BY at_cap ASC, created_at ASC
                    LIMIT 1
                ) candidate
                WHERE t.track_id = candidate.track_id
                RETURNING {TASK_RETURNING_COLUMNS_ALIASED}
                "#
                )
            };

            let pending_guard = crate::state_machine::CLAIM_PENDING_GUARD_SQL;
            let pending_sql = claim_arm_sql(&[
                &format!("{pending_guard} AND fairness_hold_until IS NULL"),
                &format!("{pending_guard} AND fairness_hold_until <= NOW()"),
            ]);
            let row = match sqlx::query(&pending_sql)
                .bind(worker_id)
                .bind(lease_token)
                .bind(lease_expires_at)
                .bind(fair_ws)
                .fetch_optional(&mut *tx)
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    let _ = tx.rollback().await;
                    return Err(TaskError::StorageError(format!(
                        "Failed to claim pending task: {e}"
                    )));
                }
            };

            let row = if row.is_some() {
                row
            } else {
                // Arms split `CLAIM_STALE_GUARD_SQL`: a null lease and an expired
                // lease each require the hold, so neither predicate is an OR.
                let stale_sql = claim_arm_sql(&[
                "status = 'processing' AND lease_expires_at IS NULL AND fairness_hold_until IS NULL",
                "status = 'processing' AND lease_expires_at IS NULL AND fairness_hold_until <= NOW()",
                "status = 'processing' AND lease_expires_at < NOW() AND fairness_hold_until IS NULL",
                "status = 'processing' AND lease_expires_at < NOW() AND fairness_hold_until <= NOW()",
            ]);
                match sqlx::query(&stale_sql)
                    .bind(worker_id)
                    .bind(lease_token)
                    .bind(lease_expires_at)
                    .bind(fair_ws)
                    .fetch_optional(&mut *tx)
                    .await
                {
                    Ok(row) => row,
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err(TaskError::StorageError(format!(
                            "Failed to claim stale processing task: {e}"
                        )));
                    }
                }
            };

            if let Some(row) = row {
                tx.commit().await.map_err(|e| {
                    TaskError::StorageError(format!("Failed to commit claim_next txn: {e}"))
                })?;
                return Ok(Some(task_from_row(&row)?));
            }
        }
        let _ = tx.rollback().await;
        Ok(None)
    }

    async fn mark_fairness_hold(&self, track_id: &str, hold_ttl: Duration) -> TaskResult<()> {
        let until = crate::lease_expires_at(Utc::now(), hold_ttl);
        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET fairness_hold_until = $2,
                updated_at = NOW()
            WHERE track_id = $1
            "#,
        )
        .bind(track_id)
        .bind(until)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to mark fairness hold: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(TaskError::TaskNotFound(track_id.to_string()));
        }
        Ok(())
    }

    async fn clear_fairness_hold(&self, track_id: &str) -> TaskResult<()> {
        sqlx::query(
            r#"
            UPDATE tasks
            SET fairness_hold_until = NULL,
                updated_at = NOW()
            WHERE track_id = $1
            "#,
        )
        .bind(track_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to clear fairness hold: {e}")))?;
        Ok(())
    }

    async fn refresh_lease(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
        lease_ttl: Duration,
    ) -> TaskResult<bool> {
        let lease_expires_at = crate::lease_expires_at(Utc::now(), lease_ttl);

        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET lease_expires_at = $4,
                updated_at = NOW()
            WHERE track_id = $1
              AND lease_owner = $2
              AND lease_token = $3
              AND status = 'processing'
            "#,
        )
        .bind(track_id)
        .bind(worker_id)
        .bind(lease_token)
        .bind(lease_expires_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to refresh lease: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn release_claim(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
    ) -> TaskResult<bool> {
        // SPEC-091 QW0: Release guard from the state machine SSOT.
        let sql = format!(
            r#"
            UPDATE tasks
            SET status = 'pending',
                lease_owner = NULL,
                lease_token = NULL,
                lease_expires_at = NULL,
                started_at = NULL,
                updated_at = NOW()
            WHERE track_id = $1
              AND lease_owner = $2
              AND lease_token = $3
              AND {}
            "#,
            crate::state_machine::RELEASE_GUARD_SQL
        );
        let result = sqlx::query(&sql)
            .bind(track_id)
            .bind(worker_id)
            .bind(lease_token)
            .execute(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to release claim: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn mark_fairness_parked(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
    ) -> TaskResult<bool> {
        // SPEC-091 R-18: release + park marker in ONE round trip. The marker
        // (migration 111) excludes the row from claim_next via the state
        // machine's CLAIM_PENDING_GUARD_SQL — idle workers stop spinning.
        let sql = format!(
            r#"
            UPDATE tasks
            SET status = 'pending',
                lease_owner = NULL,
                lease_token = NULL,
                lease_expires_at = NULL,
                started_at = NULL,
                fairness_parked_at = NOW(),
                updated_at = NOW()
            WHERE track_id = $1
              AND lease_owner = $2
              AND lease_token = $3
              AND {}
            "#,
            crate::state_machine::RELEASE_GUARD_SQL
        );
        let result = sqlx::query(&sql)
            .bind(track_id)
            .bind(worker_id)
            .bind(lease_token)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                TaskError::StorageError(format!("Failed to mark fairness-parked: {}", e))
            })?;

        Ok(result.rows_affected() > 0)
    }

    async fn clear_fairness_park(&self, track_id: &str) -> TaskResult<()> {
        sqlx::query(
            r#"
            UPDATE tasks
            SET fairness_parked_at = NULL, updated_at = NOW()
            WHERE track_id = $1
              AND status = 'pending'
              AND fairness_parked_at IS NOT NULL
            "#,
        )
        .bind(track_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to clear fairness park: {}", e)))?;
        Ok(())
    }

    async fn clear_stale_fairness_parks(&self, max_age: Duration) -> TaskResult<u64> {
        // Age is computed in SQL for clock hygiene; 0 clears all parks (boot).
        let max_age_secs = max_age.as_secs_f64();
        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET fairness_parked_at = NULL, updated_at = NOW()
            WHERE status = 'pending'
              AND fairness_parked_at IS NOT NULL
              AND fairness_parked_at <= NOW() - make_interval(secs => $1)
            "#,
        )
        .bind(max_age_secs)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!("Failed to sweep stale fairness parks: {}", e))
        })?;
        Ok(result.rows_affected())
    }

    async fn get_queue_metrics_filtered(
        &self,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> TaskResult<QueueMetrics> {
        const METRICS_STATEMENT_TIMEOUT_MS: u32 = 500;

        let mut conn = self.pool.acquire().await.map_err(|e| {
            TaskError::StorageError(format!(
                "Failed to acquire connection for queue metrics: {e}"
            ))
        })?;
        let mut tx = conn.begin().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to begin queue metrics txn: {e}"))
        })?;
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{METRICS_STATEMENT_TIMEOUT_MS}ms'"
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!(
                "Failed to set queue metrics statement_timeout: {e}"
            ))
        })?;

        let sql = queue_metrics_statement(tenant_id.is_some(), workspace_id.is_some());
        let row = match (tenant_id, workspace_id) {
            (Some(tenant), Some(workspace)) => {
                sqlx::query(&sql)
                    .bind(tenant)
                    .bind(workspace)
                    .fetch_one(&mut *tx)
                    .await
            }
            (Some(tenant), None) => sqlx::query(&sql).bind(tenant).fetch_one(&mut *tx).await,
            (None, Some(workspace)) => sqlx::query(&sql).bind(workspace).fetch_one(&mut *tx).await,
            (None, None) => {
                sqlx::query(&sql)
                    .bind(tenant_id)
                    .bind(workspace_id)
                    .fetch_one(&mut *tx)
                    .await
            }
        };
        let row = match row {
            Ok(row) => {
                tx.commit().await.map_err(|e| {
                    TaskError::StorageError(format!("Failed to commit queue metrics txn: {e}"))
                })?;
                row
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(TaskError::StorageError(format!(
                    "Failed to get queue metrics: {e}"
                )));
            }
        };

        let pending_count = row.get::<i64, _>("pending_count") as u64;
        let processing_count = row.get::<i64, _>("processing_count") as u64;
        let avg_wait_time_seconds = row.get::<f64, _>("avg_wait_seconds");
        let max_wait_time_seconds = row.get::<f64, _>("max_wait_seconds");
        let recent_completed = row.get::<i64, _>("recent_completed") as u64;

        let throughput_per_minute = recent_completed as f64 / 5.0;

        let estimated_queue_time_seconds = if throughput_per_minute > 0.0 {
            (pending_count as f64 / throughput_per_minute) * 60.0
        } else if avg_wait_time_seconds > 0.0 {
            pending_count as f64 * avg_wait_time_seconds
        } else {
            0.0
        };

        let max_workers = task_max_workers_from_env();
        let active_workers = processing_count.min(max_workers as u64) as u32;
        let worker_utilization = ((active_workers as f64 / max_workers as f64) * 100.0) as u8;

        Ok(QueueMetrics {
            pending_count,
            processing_count,
            active_workers,
            max_workers,
            worker_utilization,
            avg_wait_time_seconds,
            max_wait_time_seconds,
            throughput_per_minute,
            estimated_queue_time_seconds,
            rate_limited: QueueMetrics::compute_rate_limited(
                pending_count,
                active_workers,
                max_workers,
                throughput_per_minute,
            ),
            timestamp: chrono::Utc::now(),
        })
    }

    async fn prune_terminal_tasks(&self, older_than_days: u32) -> TaskResult<u64> {
        let days = i32::try_from(older_than_days.max(1)).map_err(|_| {
            TaskError::StorageError("older_than_days exceeds i32 range".to_string())
        })?;

        let result = sqlx::query(
            r#"
            DELETE FROM tasks
            WHERE status IN ('indexed', 'failed', 'cancelled')
              AND completed_at IS NOT NULL
              AND completed_at < NOW() - make_interval(days => $1)
            "#,
        )
        .bind(days)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to prune terminal tasks: {e}")))?;

        // SPEC-090 F-090-13: detach empty month partitions older than retention.
        let _ = sqlx::query_scalar::<_, i32>("SELECT edgequake_detach_old_task_partitions($1)")
            .bind(days)
            .fetch_optional(&*self.pool)
            .await;

        Ok(result.rows_affected())
    }

    async fn count_pending_older_than(&self, created_at: DateTime<Utc>) -> TaskResult<u64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM tasks
            WHERE status = 'pending' AND created_at < $1
            "#,
        )
        .bind(created_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to count pending ahead: {e}")))?;
        Ok(count.max(0) as u64)
    }

    async fn count_completed_within(&self, window: Duration) -> TaskResult<u64> {
        let secs = i32::try_from(window.as_secs().max(1))
            .map_err(|_| TaskError::StorageError("window exceeds i32 seconds".to_string()))?;
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM tasks
            WHERE completed_at IS NOT NULL
              AND completed_at >= NOW() - make_interval(secs => $1)
            "#,
        )
        .bind(secs)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to count recent completions: {e}")))?;
        Ok(count.max(0) as u64)
    }

    async fn pending_queue_ahead_batch(
        &self,
        track_ids: &[String],
    ) -> TaskResult<Vec<(String, u64)>> {
        if track_ids.is_empty() {
            return Ok(Vec::new());
        }
        // Rank is the number of pending rows strictly before this one in
        // (created_at, track_id) order. Two index-only counts. A window over
        // every pending row sorts the whole queue to answer a page of ids.
        let rows = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT t.track_id,
              (
                SELECT COUNT(*)::bigint FROM tasks p
                WHERE p.status = 'pending' AND p.created_at < t.created_at
              ) + (
                SELECT COUNT(*)::bigint FROM tasks p
                WHERE p.status = 'pending'
                  AND p.created_at = t.created_at
                  AND p.track_id < t.track_id
              ) AS ahead
            FROM tasks t
            WHERE t.track_id = ANY($1)
              AND t.status = 'pending'
            "#,
        )
        .bind(track_ids)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!("Failed to batch pending queue ranks: {e}"))
        })?;
        Ok(rows
            .into_iter()
            .map(|(id, ahead)| (id, ahead.max(0) as u64))
            .collect())
    }
}

#[cfg(feature = "postgres")]
impl PostgresTaskStorage {
    async fn get_estimated_count(&self, filter: TaskFilter) -> TaskResult<u64> {
        let has_filters = filter.tenant_id.is_some()
            || filter.workspace_id.is_some()
            || filter.status.is_some()
            || filter.task_type.is_some();

        if !has_filters {
            let estimate: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT GREATEST(reltuples::bigint, 0)
                FROM pg_class
                WHERE oid = 'public.tasks'::regclass
                "#,
            )
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to estimate task count: {e}")))?;
            if let Some(n) = estimate {
                if n >= 0 {
                    return Ok(n as u64);
                }
            }
        }

        let mut query = String::from("SELECT COUNT(*) FROM tasks WHERE 1=1");
        let mut param_count = 0;

        if filter.tenant_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND tenant_id = ${}", param_count));
        }
        if filter.workspace_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND workspace_id = ${}", param_count));
        }
        if filter.status.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND status = ${}", param_count));
        }
        if filter.task_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND task_type = ${}", param_count));
        }

        let mut conn = self.pool.acquire().await.map_err(|e| {
            TaskError::StorageError(format!("Failed to acquire connection for task count: {e}"))
        })?;
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to begin count txn: {e}")))?;
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{LIST_STATEMENT_TIMEOUT_MS}ms'"
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!("Failed to set count statement_timeout: {e}"))
        })?;

        let mut query_builder = sqlx::query(&query);
        if let Some(tenant_id) = &filter.tenant_id {
            query_builder = query_builder.bind(tenant_id);
        }
        if let Some(workspace_id) = &filter.workspace_id {
            query_builder = query_builder.bind(workspace_id);
        }
        if let Some(status) = &filter.status {
            query_builder = query_builder.bind(status.to_string());
        }
        if let Some(task_type) = &filter.task_type {
            query_builder = query_builder.bind(task_type.to_string());
        }

        let row = match query_builder.fetch_one(&mut *tx).await {
            Ok(row) => {
                tx.commit().await.map_err(|e| {
                    TaskError::StorageError(format!("Failed to commit count txn: {e}"))
                })?;
                row
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(TaskError::StorageError(format!(
                    "Failed to count tasks: {e}"
                )));
            }
        };

        Ok(row.get::<i64, _>(0) as u64)
    }
}

#[cfg(feature = "postgres")]
impl std::str::FromStr for crate::types::TaskType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "upload" => Ok(crate::types::TaskType::Upload),
            "insert" => Ok(crate::types::TaskType::Insert),
            "scan" => Ok(crate::types::TaskType::Scan),
            "reindex" => Ok(crate::types::TaskType::Reindex),
            "pdf_processing" => Ok(crate::types::TaskType::PdfProcessing),
            "knowledge_injection" => Ok(crate::types::TaskType::KnowledgeInjection),
            "deletion" => Ok(crate::types::TaskType::Deletion),
            "batch_deletion" => Ok(crate::types::TaskType::BatchDeletion),
            "workspace_wipe" => Ok(crate::types::TaskType::WorkspaceWipe),
            _ => Err(format!("Invalid task type: {}", s)),
        }
    }
}

#[cfg(feature = "postgres")]
impl std::str::FromStr for crate::types::TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(crate::types::TaskStatus::Pending),
            "processing" => Ok(crate::types::TaskStatus::Processing),
            "indexed" => Ok(crate::types::TaskStatus::Indexed),
            "failed" => Ok(crate::types::TaskStatus::Failed),
            "cancelled" => Ok(crate::types::TaskStatus::Cancelled),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

#[cfg(all(test, feature = "postgres"))]
mod queue_metrics_sql_tests {
    use super::queue_metrics_statement;

    #[test]
    fn scoped_queue_metrics_require_the_caller_predicate() {
        let both = queue_metrics_statement(true, true);
        assert!(both.contains("tenant_id = $1 AND workspace_id = $2"));
        assert!(!both.contains("IS NULL"));
        let tenant = queue_metrics_statement(true, false);
        assert!(tenant.contains("WHERE tenant_id = $1"));
        assert!(!tenant.contains("workspace_id"));
        let open = queue_metrics_statement(false, false);
        assert!(open.contains("$1::uuid IS NULL OR tenant_id = $1"));
    }

    #[test]
    fn fair_claim_samples_require_each_predicate() {
        let pending = super::FAIR_PENDING_SAMPLE_SQL;
        assert!(pending.contains("fairness_hold_until IS NULL"));
        assert!(pending.contains("fairness_hold_until <= NOW()"));
        assert!(!pending.contains("fairness_hold_until IS NULL OR"));
        let stale = super::FAIR_STALE_SAMPLE_SQL;
        assert!(stale.contains("lease_expires_at IS NULL"));
        assert!(stale.contains("lease_expires_at < NOW()"));
        assert!(!stale.contains("lease_expires_at IS NULL OR"));
        assert!(!stale.contains("fairness_hold_until IS NULL OR"));
    }
}
