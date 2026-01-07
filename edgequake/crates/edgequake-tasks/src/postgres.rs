//! PostgreSQL task storage implementation.

#[cfg(feature = "postgres")]
use crate::{
    error::{TaskError, TaskResult},
    storage::*,
    types::Task,
};
#[cfg(feature = "postgres")]
use sqlx::{PgPool, Row};
#[cfg(feature = "postgres")]
use std::sync::Arc;

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
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, created_at, updated_at,
                started_at, completed_at, error_message, retry_count,
                max_retries, task_data, metadata, progress, result
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(&task.track_id)
        .bind(task.task_type.to_string())
        .bind(task.status.to_string())
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(&task.error_message)
        .bind(task.retry_count)
        .bind(task.max_retries)
        .bind(&task.task_data)
        .bind(&task.metadata)
        .bind(serde_json::to_value(&task.progress)?)
        .bind(&task.result)
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to create task: {}", e)))?;

        Ok(())
    }

    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>> {
        let row = sqlx::query(
            r#"
            SELECT 
                track_id, task_type, status, created_at, updated_at,
                started_at, completed_at, error_message, retry_count,
                max_retries, task_data, metadata, progress, result
            FROM tasks
            WHERE track_id = $1
            "#,
        )
        .bind(track_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to fetch task: {}", e)))?;

        if let Some(row) = row {
            let task = Task {
                track_id: row.get("track_id"),
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
                task_data: row.get("task_data"),
                metadata: row.get("metadata"),
                progress: row
                    .get::<Option<serde_json::Value>, _>("progress")
                    .and_then(|v| serde_json::from_value(v).ok()),
                result: row.get("result"),
            };
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    async fn update_task(&self, task: &Task) -> TaskResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE tasks SET
                status = $2,
                updated_at = $3,
                started_at = $4,
                completed_at = $5,
                error_message = $6,
                retry_count = $7,
                progress = $8,
                result = $9
            WHERE track_id = $1
            "#,
        )
        .bind(&task.track_id)
        .bind(task.status.to_string())
        .bind(task.updated_at)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(&task.error_message)
        .bind(task.retry_count)
        .bind(serde_json::to_value(&task.progress)?)
        .bind(&task.result)
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
        // Build query with filters
        let mut query = String::from(
            "SELECT 
                track_id, task_type, status, created_at, updated_at,
                started_at, completed_at, error_message, retry_count,
                max_retries, task_data, metadata, progress, result
            FROM tasks WHERE 1=1",
        );

        if filter.status.is_some() {
            query.push_str(" AND status = $1");
        }
        if filter.task_type.is_some() {
            query.push_str(if filter.status.is_some() {
                " AND task_type = $2"
            } else {
                " AND task_type = $1"
            });
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

        // Execute query
        let mut query_builder = sqlx::query(&query);
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
            .filter_map(|row| {
                Some(Task {
                    track_id: row.get("track_id"),
                    task_type: row.get::<String, _>("task_type").parse().ok()?,
                    status: row.get::<String, _>("status").parse().ok()?,
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
                    task_data: row.get("task_data"),
                    metadata: row.get("metadata"),
                    progress: row
                        .get::<Option<serde_json::Value>, _>("progress")
                        .and_then(|v| serde_json::from_value(v).ok()),
                    result: row.get("result"),
                })
            })
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

    async fn get_statistics(&self) -> TaskResult<TaskStatistics> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'processing') as processing,
                COUNT(*) FILTER (WHERE status = 'indexed') as indexed,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled,
                COUNT(*) as total
            FROM tasks
            "#,
        )
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
}

#[cfg(feature = "postgres")]
impl PostgresTaskStorage {
    async fn get_total_count(&self, filter: TaskFilter) -> TaskResult<u64> {
        let mut query = String::from("SELECT COUNT(*) FROM tasks WHERE 1=1");

        if filter.status.is_some() {
            query.push_str(" AND status = $1");
        }
        if filter.task_type.is_some() {
            query.push_str(if filter.status.is_some() {
                " AND task_type = $2"
            } else {
                " AND task_type = $1"
            });
        }

        let mut query_builder = sqlx::query(&query);
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
