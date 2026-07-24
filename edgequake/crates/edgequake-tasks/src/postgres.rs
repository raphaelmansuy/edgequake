//! PostgreSQL task storage implementation.

#[cfg(feature = "postgres")]
use crate::{
    error::{TaskError, TaskResult},
    storage::*,
    types::Task,
};
#[cfg(feature = "postgres")]
use chrono::Utc;
#[cfg(feature = "postgres")]
use sqlx::{postgres::PgRow, PgPool, Row};
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
    payload, result, lease_owner, lease_token, lease_expires_at
"#;

/// RETURNING list for `UPDATE tasks t … FROM candidate` — must qualify as `t.*`
/// because `candidate.track_id` makes bare `track_id` ambiguous (Postgres).
#[cfg(feature = "postgres")]
const TASK_RETURNING_COLUMNS_ALIASED: &str = r#"
    t.track_id, t.tenant_id, t.workspace_id, t.task_type, t.status, t.created_at, t.updated_at,
    t.started_at, t.completed_at, t.error_message, t.error, t.retry_count,
    t.max_retries, t.consecutive_timeout_failures, t.circuit_breaker_tripped,
    t.payload, t.result, t.lease_owner, t.lease_token, t.lease_expires_at
"#;

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
    let progress = payload.get("progress").cloned().and_then(|v| {
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
    })
}

#[cfg(feature = "postgres")]
/// PostgreSQL task storage
#[derive(Debug, Clone)]
pub struct PostgresTaskStorage {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresTaskStorage {
    /// Create a new PostgreSQL storage
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Create from an Arc pool
    pub fn from_arc(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl TaskStorage for PostgresTaskStorage {
    async fn create_task(&self, task: &Task) -> TaskResult<()> {
        // WHY: The database schema uses `payload` column (JSONB) to store task_data,
        // metadata, and progress together. This is different from the Task struct
        // which separates these fields. We combine them into payload for storage.
        //
        // Database columns (from migration):
        // - payload: JSONB NOT NULL - stores task_data + metadata + progress
        // - result: JSONB - stores result on completion
        //
        // This mapping allows the Task struct to maintain clean separation while
        // the database uses a single JSONB column for flexibility.
        let payload = serde_json::json!({
            "task_data": task.task_data,
            "metadata": task.metadata,
            "progress": task.progress,
        });

        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, tenant_id, workspace_id, task_type, status, created_at, updated_at,
                started_at, completed_at, error_message, error, retry_count,
                max_retries, consecutive_timeout_failures, circuit_breaker_tripped,
                payload, result, lease_owner, lease_token, lease_expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
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
        .bind(&task.result)
        .bind(&task.lease_owner)
        .bind(task.lease_token)
        .bind(task.lease_expires_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to create task: {}", e)))?;

        Ok(())
    }

    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>> {
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

    /// Lightweight heartbeat: only update `updated_at` column.
    ///
    /// WHY: This is ~10x cheaper than a full `update_task` because it doesn't
    /// serialize/deserialize the JSONB payload column. Workers call this every
    /// 60 seconds during long-running LLM extraction to signal liveness.
    async fn touch_task(&self, track_id: &str) -> TaskResult<()> {
        sqlx::query("UPDATE tasks SET updated_at = NOW() WHERE track_id = $1")
            .bind(track_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to touch task: {}", e)))?;
        Ok(())
    }

    async fn update_task(&self, task: &Task) -> TaskResult<()> {
        // WHY: Update payload JSONB with combined task_data, metadata, progress
        // We only update the progress inside payload on updates (task_data is immutable)
        let payload = serde_json::json!({
            "task_data": task.task_data,
            "metadata": task.metadata,
            "progress": task.progress,
        });

        let result = sqlx::query(
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
                result = $12,
                lease_owner = $13,
                lease_token = $14,
                lease_expires_at = $15
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
        .bind(&task.result)
        .bind(&task.lease_owner)
        .bind(task.lease_token)
        .bind(task.lease_expires_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to update task: {}", e)))?;

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
        // WHY: Query uses `payload` column instead of separate task_data, metadata, progress columns
        // The payload JSONB contains all three fields combined
        let mut query = format!("SELECT {TASK_SELECT_COLUMNS} FROM tasks WHERE 1=1");

        let mut param_count = 0;

        // CRITICAL: Filter by tenant_id and workspace_id for multi-tenancy isolation
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

        // Add sorting
        let sort_field = match pagination.sort_by {
            SortField::CreatedAt => "created_at",
            SortField::UpdatedAt => "updated_at",
        };
        let sort_order = match pagination.order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        query.push_str(&format!(" ORDER BY {} {}", sort_field, sort_order));

        // Add pagination
        let offset = (pagination.page - 1) * pagination.page_size;
        query.push_str(&format!(
            " LIMIT {} OFFSET {}",
            pagination.page_size, offset
        ));

        // Execute query with dynamic binding
        let mut query_builder = sqlx::query(&query);

        // Bind parameters in the same order as they appear in the query
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

        let rows = query_builder
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to list tasks: {}", e)))?;

        let tasks: Vec<Task> = rows
            .into_iter()
            .filter_map(|row| task_from_row(&row).ok())
            .collect();

        // Get total count
        let total = self.get_total_count(filter).await?;
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
        // Without this, statistics would leak across tenant boundaries
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

        let row = sqlx_query
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to get statistics: {}", e)))?;

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
        // SPEC-057 P2: Convert (`pdf_processing`) or follow-on Insert ingest.
        let sql = format!(
            r#"
            SELECT {TASK_SELECT_COLUMNS}
            FROM tasks
            WHERE workspace_id = $1
              AND status IN ('pending', 'processing')
              AND (
                    (task_type = 'pdf_processing'
                     AND payload->'task_data'->>'pdf_id' = $2)
                 OR (task_type = 'insert'
                     AND payload->'task_data'->'metadata'->>'pdf_id' = $2)
              )
            ORDER BY created_at DESC
            LIMIT 1
            "#
        );
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
              AND task_type = 'insert'
              AND status IN ('pending', 'processing')
              AND payload->'task_data'->'metadata'->>'pdf_id' = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#
        );
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

    async fn claim_next(&self, worker_id: &str, lease_ttl: Duration) -> TaskResult<Option<Task>> {
        let lease_token = Uuid::new_v4();
        let lease_expires_at = crate::lease_expires_at(Utc::now(), lease_ttl);

        // SPEC-084 / GH-316 / LAW-13: workspace-fair claim (least-loaded, then oldest).
        // Prefer workspaces with fewer active leases so a backlog in WS-A cannot
        // starve WS-B; still SKIP LOCKED safe for concurrent workers.
        let sql = format!(
            r#"
            WITH claimable AS (
                SELECT track_id, workspace_id, created_at
                FROM tasks
                WHERE status = 'pending'
                   OR (
                        status = 'processing'
                        AND (lease_expires_at IS NULL OR lease_expires_at < NOW())
                   )
            ),
            ws_load AS (
                SELECT workspace_id, COUNT(*)::bigint AS active_count
                FROM tasks
                WHERE status = 'processing'
                  AND lease_expires_at IS NOT NULL
                  AND lease_expires_at >= NOW()
                GROUP BY workspace_id
            ),
            fair_workspace AS (
                SELECT c.workspace_id
                FROM claimable c
                LEFT JOIN ws_load w ON w.workspace_id = c.workspace_id
                GROUP BY c.workspace_id
                ORDER BY COALESCE(MAX(w.active_count), 0) ASC, MIN(c.created_at) ASC
                LIMIT 1
            ),
            candidate AS (
                SELECT t.track_id
                FROM tasks t
                INNER JOIN fair_workspace fw ON t.workspace_id = fw.workspace_id
                WHERE t.status = 'pending'
                   OR (
                        t.status = 'processing'
                        AND (t.lease_expires_at IS NULL OR t.lease_expires_at < NOW())
                   )
                ORDER BY t.created_at ASC
                FOR UPDATE OF t SKIP LOCKED
                LIMIT 1
            )
            UPDATE tasks t
            SET status = 'processing',
                lease_owner = $1,
                lease_token = $2,
                lease_expires_at = $3,
                started_at = COALESCE(t.started_at, NOW()),
                updated_at = NOW(),
                completed_at = NULL
            FROM candidate
            WHERE t.track_id = candidate.track_id
            RETURNING {TASK_RETURNING_COLUMNS_ALIASED}
            "#
        );

        let row = sqlx::query(&sql)
            .bind(worker_id)
            .bind(lease_token)
            .bind(lease_expires_at)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to claim next task: {}", e)))?;

        match row {
            Some(row) => Ok(Some(task_from_row(&row)?)),
            None => Ok(None),
        }
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
        let result = sqlx::query(
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
              AND status = 'processing'
            "#,
        )
        .bind(track_id)
        .bind(worker_id)
        .bind(lease_token)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to release claim: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_queue_metrics_filtered(
        &self,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> TaskResult<QueueMetrics> {
        // OODA-04: Add tenant/workspace filtering for multi-tenant isolation
        //
        // WHY: Queue metrics MUST be scoped to the current tenant/workspace.
        // Without this filter, users see processing activity from ALL workspaces,
        // which is a privacy violation and causes user confusion.
        //
        // The filter uses ($1::uuid IS NULL OR tenant_id = $1) pattern to make
        // parameters optional - if None is passed, no filtering is applied.
        let row = sqlx::query(
            r#"
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
            WHERE ($1::uuid IS NULL OR tenant_id = $1)
              AND ($2::uuid IS NULL OR workspace_id = $2)
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to get queue metrics: {}", e)))?;

        let pending_count = row.get::<i64, _>("pending_count") as u64;
        let processing_count = row.get::<i64, _>("processing_count") as u64;
        let avg_wait_time_seconds = row.get::<f64, _>("avg_wait_seconds");
        let max_wait_time_seconds = row.get::<f64, _>("max_wait_seconds");
        let recent_completed = row.get::<i64, _>("recent_completed") as u64;

        // Calculate throughput (docs per minute over last 5 minutes)
        let throughput_per_minute = recent_completed as f64 / 5.0;

        // Estimate queue time
        let estimated_queue_time_seconds = if throughput_per_minute > 0.0 {
            (pending_count as f64 / throughput_per_minute) * 60.0
        } else if avg_wait_time_seconds > 0.0 {
            pending_count as f64 * avg_wait_time_seconds
        } else {
            0.0
        };

        // Worker utilization (assuming 4 max workers)
        let max_workers = 4u32;
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
            rate_limited: false, // TODO: Track rate limiting separately
            timestamp: chrono::Utc::now(),
        })
    }
}

#[cfg(feature = "postgres")]
impl PostgresTaskStorage {
    async fn get_total_count(&self, filter: TaskFilter) -> TaskResult<u64> {
        let mut query = String::from("SELECT COUNT(*) FROM tasks WHERE 1=1");

        let mut param_count = 0;

        // CRITICAL: Filter by tenant_id and workspace_id for multi-tenancy isolation
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

        let mut query_builder = sqlx::query(&query);

        // Bind parameters in the same order as they appear in the query
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

        let row = query_builder
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| TaskError::StorageError(format!("Failed to count tasks: {}", e)))?;

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
