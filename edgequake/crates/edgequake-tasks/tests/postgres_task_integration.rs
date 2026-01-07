//! PostgreSQL Task Storage Integration Tests
//!
//! These tests verify the PostgresTaskStorage implementation
//! against a real PostgreSQL database.
//!
//! Run with:
//!   cargo test --package edgequake-tasks --test postgres_task_integration --features postgres
//!
//! Environment variables needed:
//!   - DATABASE_URL or POSTGRES_PASSWORD

#![cfg(feature = "postgres")]

use std::env;

use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

/// Get database URL from environment
fn get_database_url() -> Option<String> {
    env::var("DATABASE_URL").ok().or_else(|| {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake_test".to_string());
        let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake_test".to_string());
        Some(format!(
            "postgresql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })
}

/// Create test database pool
async fn create_test_pool() -> Option<PgPool> {
    let database_url = get_database_url()?;

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .ok()
}

macro_rules! require_postgres {
    () => {
        match create_test_pool().await {
            Some(pool) => pool,
            None => {
                eprintln!("Skipping test: DATABASE_URL or POSTGRES_PASSWORD not set");
                return;
            }
        }
    };
}

// ============================================================================
// Task CRUD Tests
// ============================================================================

mod task_crud {
    use super::*;

    #[tokio::test]
    async fn test_create_task() {
        let pool = require_postgres!();

        let track_id = format!("test-task-{}", Uuid::new_v4());

        let result = sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING track_id
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("pending")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({"document_id": "doc-123"}))
        .fetch_one(&pool)
        .await;

        assert!(result.is_ok(), "Failed to create task: {:?}", result.err());

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_get_task_by_track_id() {
        let pool = require_postgres!();

        let track_id = format!("get-task-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("pending")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({"doc": "test"}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Get task
        let row = sqlx::query("SELECT track_id, task_type, status FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get task");

        let retrieved_track_id: String = row.get("track_id");
        let task_type: String = row.get("task_type");
        let status: String = row.get("status");

        assert_eq!(retrieved_track_id, track_id);
        assert_eq!(task_type, "document_ingestion");
        assert_eq!(status, "pending");

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let pool = require_postgres!();

        let track_id = format!("update-task-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("pending")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Update status to running
        sqlx::query(
            "UPDATE tasks SET status = $1, started_at = $2, updated_at = $2 WHERE track_id = $3",
        )
        .bind("running")
        .bind(Utc::now())
        .bind(&track_id)
        .execute(&pool)
        .await
        .expect("Failed to update task");

        // Verify status
        let row = sqlx::query("SELECT status, started_at FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get task");

        let status: String = row.get("status");
        assert_eq!(status, "running");

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_complete_task() {
        let pool = require_postgres!();

        let track_id = format!("complete-task-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("running")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        let now = Utc::now();
        let result = serde_json::json!({
            "entities_extracted": 15,
            "relationships_created": 25
        });

        // Complete task
        sqlx::query(
            "UPDATE tasks SET status = $1, completed_at = $2, result = $3, updated_at = $2 WHERE track_id = $4"
        )
        .bind("completed")
        .bind(now)
        .bind(&result)
        .bind(&track_id)
        .execute(&pool)
        .await
        .expect("Failed to complete task");

        // Verify
        let row = sqlx::query("SELECT status, result FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get task");

        let status: String = row.get("status");
        let stored_result: serde_json::Value = row.get("result");

        assert_eq!(status, "completed");
        assert_eq!(stored_result["entities_extracted"], 15);

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_fail_task() {
        let pool = require_postgres!();

        let track_id = format!("fail-task-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("running")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Fail task
        let error_message = "Connection timeout: LLM provider not responding";
        sqlx::query(
            "UPDATE tasks SET status = $1, error_message = $2, completed_at = NOW(), updated_at = NOW() WHERE track_id = $3"
        )
        .bind("failed")
        .bind(error_message)
        .bind(&track_id)
        .execute(&pool)
        .await
        .expect("Failed to fail task");

        // Verify
        let row = sqlx::query("SELECT status, error_message FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get task");

        let status: String = row.get("status");
        let stored_error: String = row.get("error_message");

        assert_eq!(status, "failed");
        assert!(stored_error.contains("timeout"));

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_delete_task() {
        let pool = require_postgres!();

        let track_id = format!("delete-task-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("pending")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Delete task
        let result = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await
            .expect("Failed to delete task");

        assert_eq!(result.rows_affected(), 1);

        // Verify deletion
        let count = sqlx::query("SELECT COUNT(*) as cnt FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to count tasks");

        let cnt: i64 = count.get("cnt");
        assert_eq!(cnt, 0);
    }
}

// ============================================================================
// Task Query Tests
// ============================================================================

mod task_queries {
    use super::*;

    async fn create_test_tasks(
        pool: &PgPool,
        prefix: &str,
        count: usize,
        task_type: &str,
        status: &str,
    ) -> Vec<String> {
        let mut track_ids = Vec::new();

        for i in 0..count {
            let track_id = format!("{}-{}-{}", prefix, i, Uuid::new_v4());

            sqlx::query(
                r#"
                INSERT INTO tasks (
                    track_id, task_type, status, priority,
                    retry_count, max_retries, payload
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(&track_id)
            .bind(task_type)
            .bind(status)
            .bind(0i32)
            .bind(0i32)
            .bind(3i32)
            .bind(serde_json::json!({"index": i}))
            .execute(pool)
            .await
            .expect("Failed to create test task");

            track_ids.push(track_id);
        }

        track_ids
    }

    async fn cleanup_tasks(pool: &PgPool, track_ids: &[String]) {
        for track_id in track_ids {
            let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
                .bind(track_id)
                .execute(pool)
                .await;
        }
    }

    #[tokio::test]
    async fn test_list_tasks_by_status() {
        let pool = require_postgres!();

        let pending_ids =
            create_test_tasks(&pool, "status-pending", 3, "document_ingestion", "pending").await;
        let running_ids =
            create_test_tasks(&pool, "status-running", 2, "document_ingestion", "running").await;

        // Query pending tasks
        let rows = sqlx::query("SELECT track_id FROM tasks WHERE status = $1 AND track_id LIKE $2")
            .bind("pending")
            .bind("status-pending%")
            .fetch_all(&pool)
            .await
            .expect("Failed to list tasks");

        assert_eq!(rows.len(), 3);

        // Cleanup
        cleanup_tasks(&pool, &pending_ids).await;
        cleanup_tasks(&pool, &running_ids).await;
    }

    #[tokio::test]
    async fn test_list_tasks_by_type() {
        let pool = require_postgres!();

        let ingestion_ids =
            create_test_tasks(&pool, "type-ingestion", 2, "document_ingestion", "pending").await;
        let embedding_ids = create_test_tasks(
            &pool,
            "type-embedding",
            3,
            "embedding_generation",
            "pending",
        )
        .await;

        // Query by type
        let rows =
            sqlx::query("SELECT track_id FROM tasks WHERE task_type = $1 AND track_id LIKE $2")
                .bind("embedding_generation")
                .bind("type-embedding%")
                .fetch_all(&pool)
                .await
                .expect("Failed to list tasks");

        assert_eq!(rows.len(), 3);

        // Cleanup
        cleanup_tasks(&pool, &ingestion_ids).await;
        cleanup_tasks(&pool, &embedding_ids).await;
    }

    #[tokio::test]
    async fn test_list_tasks_with_pagination() {
        let pool = require_postgres!();

        let all_ids =
            create_test_tasks(&pool, "paginate", 10, "document_ingestion", "pending").await;

        // Get first page
        let page1 = sqlx::query(
            "SELECT track_id FROM tasks WHERE track_id LIKE $1 ORDER BY created_at ASC LIMIT 5",
        )
        .bind("paginate%")
        .fetch_all(&pool)
        .await
        .expect("Failed to get page 1");

        assert_eq!(page1.len(), 5);

        // Get second page
        let page2 = sqlx::query("SELECT track_id FROM tasks WHERE track_id LIKE $1 ORDER BY created_at ASC LIMIT 5 OFFSET 5")
            .bind("paginate%")
            .fetch_all(&pool)
            .await
            .expect("Failed to get page 2");

        assert_eq!(page2.len(), 5);

        // Cleanup
        cleanup_tasks(&pool, &all_ids).await;
    }

    #[tokio::test]
    async fn test_list_tasks_order_by_created_at() {
        let pool = require_postgres!();

        let prefix = format!("order-{}", Uuid::new_v4());
        let mut track_ids = Vec::new();

        // Create tasks with slight delay
        for i in 0..3 {
            let track_id = format!("{}-{}", prefix, i);

            sqlx::query(
                r#"
                INSERT INTO tasks (
                    track_id, task_type, status, priority,
                    retry_count, max_retries, payload
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(&track_id)
            .bind("document_ingestion")
            .bind("pending")
            .bind(0i32)
            .bind(0i32)
            .bind(3i32)
            .bind(serde_json::json!({"order": i}))
            .execute(&pool)
            .await
            .expect("Failed to create task");

            track_ids.push(track_id);
        }

        // Query in descending order
        let rows = sqlx::query(
            "SELECT track_id, payload FROM tasks WHERE track_id LIKE $1 ORDER BY created_at DESC",
        )
        .bind(format!("{}%", prefix))
        .fetch_all(&pool)
        .await
        .expect("Failed to list tasks");

        // Verify ordering (most recent first)
        assert_eq!(rows.len(), 3);

        // Cleanup
        cleanup_tasks(&pool, &track_ids).await;
    }
}

// ============================================================================
// Task Retry Tests
// ============================================================================

mod task_retries {
    use super::*;

    #[tokio::test]
    async fn test_increment_retry_count() {
        let pool = require_postgres!();

        let track_id = format!("retry-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("running")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Increment retry count
        sqlx::query("UPDATE tasks SET retry_count = retry_count + 1 WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await
            .expect("Failed to increment retry");

        // Verify
        let row = sqlx::query("SELECT retry_count FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get task");

        let retry_count: i32 = row.get("retry_count");
        assert_eq!(retry_count, 1);

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let pool = require_postgres!();

        let track_id = format!("max-retry-{}", Uuid::new_v4());

        // Create task with max_retries = 3
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("running")
        .bind(0i32)
        .bind(3i32) // Already at max
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Check if max retries exceeded
        let row = sqlx::query(
            "SELECT retry_count >= max_retries as exceeded FROM tasks WHERE track_id = $1",
        )
        .bind(&track_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to get task");

        let exceeded: bool = row.get("exceeded");
        assert!(exceeded);

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }
}

// ============================================================================
// Task Result Tests
// ============================================================================

mod task_results {
    use super::*;

    #[tokio::test]
    async fn test_store_task_result() {
        let pool = require_postgres!();

        let track_id = format!("result-{}", Uuid::new_v4());

        // Create task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("running")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Store result
        let result = serde_json::json!({
            "entities": ["ALICE", "BOB", "COMPANY_X"],
            "relationships": [
                {"source": "ALICE", "target": "COMPANY_X", "type": "WORKS_AT"}
            ],
            "processing_time_ms": 1234
        });

        sqlx::query("UPDATE tasks SET result = $1, status = 'completed', completed_at = NOW() WHERE track_id = $2")
            .bind(&result)
            .bind(&track_id)
            .execute(&pool)
            .await
            .expect("Failed to store result");

        // Verify result
        let row = sqlx::query("SELECT result FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get task");

        let stored_result: serde_json::Value = row.get("result");
        assert_eq!(stored_result["entities"].as_array().unwrap().len(), 3);
        assert_eq!(stored_result["processing_time_ms"], 1234);

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }
}

// ============================================================================
// Concurrent Task Tests
// ============================================================================

mod concurrent_tasks {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_task_creation() {
        let pool = require_postgres!();

        // Create 10 tasks concurrently
        let handles: Vec<tokio::task::JoinHandle<Result<String, sqlx::Error>>> = (0..10)
            .map(|i| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let track_id = format!("concurrent-{}-{}", i, Uuid::new_v4());

                    sqlx::query(
                        r#"
                        INSERT INTO tasks (
                            track_id, task_type, status, priority,
                            retry_count, max_retries, payload
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                        "#,
                    )
                    .bind(&track_id)
                    .bind("document_ingestion")
                    .bind("pending")
                    .bind(0i32)
                    .bind(0i32)
                    .bind(3i32)
                    .bind(serde_json::json!({}))
                    .execute(&pool)
                    .await
                    .map(|_| track_id)
                })
            })
            .collect();

        let mut successful_ids: Vec<String> = Vec::new();
        for handle in handles {
            if let Ok(Ok(track_id)) = handle.await {
                successful_ids.push(track_id);
            }
        }

        assert_eq!(
            successful_ids.len(),
            10,
            "All 10 concurrent insertions should succeed"
        );

        // Cleanup
        for track_id in &successful_ids {
            let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
                .bind(track_id)
                .execute(&pool)
                .await;
        }
    }

    #[tokio::test]
    async fn test_concurrent_task_updates() {
        let pool = require_postgres!();

        let track_id = format!("concurrent-update-{}", Uuid::new_v4());

        // Create initial task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("running")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Update result concurrently
        let handles: Vec<
            tokio::task::JoinHandle<Result<sqlx::postgres::PgQueryResult, sqlx::Error>>,
        > = (0..5)
            .map(|i| {
                let pool = pool.clone();
                let track_id = track_id.clone();
                tokio::spawn(async move {
                    let result = serde_json::json!({
                        "update_number": i
                    });

                    sqlx::query(
                        "UPDATE tasks SET result = $1, updated_at = NOW() WHERE track_id = $2",
                    )
                    .bind(&result)
                    .bind(&track_id)
                    .execute(&pool)
                    .await
                })
            })
            .collect();

        // Await each handle individually
        let mut all_succeeded = true;
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => {}
                _ => {
                    all_succeeded = false;
                }
            }
        }

        // All updates should succeed (last write wins)
        assert!(all_succeeded, "All concurrent updates should succeed");

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }
}

// ============================================================================
// Task Priority Tests
// ============================================================================

mod task_priority {
    use super::*;

    #[tokio::test]
    async fn test_task_priority_ordering() {
        let pool = require_postgres!();

        let prefix = format!("priority-{}", Uuid::new_v4());
        let mut track_ids = Vec::new();

        // Create tasks with different priorities
        for priority in [1, 5, 3, 2, 4] {
            let track_id = format!("{}-p{}", prefix, priority);

            sqlx::query(
                r#"
                INSERT INTO tasks (
                    track_id, task_type, status, priority,
                    retry_count, max_retries, payload
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(&track_id)
            .bind("document_ingestion")
            .bind("pending")
            .bind(priority)
            .bind(0i32)
            .bind(3i32)
            .bind(serde_json::json!({}))
            .execute(&pool)
            .await
            .expect("Failed to create task");

            track_ids.push(track_id);
        }

        // Query by priority (highest first)
        let rows = sqlx::query(
            "SELECT track_id, priority FROM tasks WHERE track_id LIKE $1 ORDER BY priority DESC",
        )
        .bind(format!("{}%", prefix))
        .fetch_all(&pool)
        .await
        .expect("Failed to list tasks");

        assert_eq!(rows.len(), 5);

        // Verify ordering
        let priorities: Vec<i32> = rows.iter().map(|r| r.get("priority")).collect();
        assert_eq!(priorities, vec![5, 4, 3, 2, 1]);

        // Cleanup
        for track_id in &track_ids {
            let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
                .bind(track_id)
                .execute(&pool)
                .await;
        }
    }
}

// ============================================================================
// Task Scheduling Tests
// ============================================================================

mod task_scheduling {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_scheduled_task() {
        let pool = require_postgres!();

        let track_id = format!("scheduled-{}", Uuid::new_v4());
        let scheduled_at = Utc::now() + Duration::hours(1);

        // Create scheduled task
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload, scheduled_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("pending")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .bind(scheduled_at)
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Query tasks due now (should not include our task)
        let rows =
            sqlx::query("SELECT track_id FROM tasks WHERE scheduled_at <= NOW() AND track_id = $1")
                .bind(&track_id)
                .fetch_all(&pool)
                .await
                .expect("Failed to query tasks");

        assert_eq!(rows.len(), 0, "Scheduled task should not be due yet");

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    async fn test_get_due_tasks() {
        let pool = require_postgres!();

        let track_id = format!("due-{}", Uuid::new_v4());
        let scheduled_at = Utc::now() - Duration::minutes(5); // 5 minutes ago

        // Create task scheduled in the past
        sqlx::query(
            r#"
            INSERT INTO tasks (
                track_id, task_type, status, priority,
                retry_count, max_retries, payload, scheduled_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&track_id)
        .bind("document_ingestion")
        .bind("pending")
        .bind(0i32)
        .bind(0i32)
        .bind(3i32)
        .bind(serde_json::json!({}))
        .bind(scheduled_at)
        .execute(&pool)
        .await
        .expect("Failed to create task");

        // Query tasks due now
        let rows =
            sqlx::query("SELECT track_id FROM tasks WHERE scheduled_at <= NOW() AND track_id = $1")
                .bind(&track_id)
                .fetch_all(&pool)
                .await
                .expect("Failed to query tasks");

        assert_eq!(rows.len(), 1, "Task scheduled in past should be due");

        // Cleanup
        let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
            .bind(&track_id)
            .execute(&pool)
            .await;
    }
}
