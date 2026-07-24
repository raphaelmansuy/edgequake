//! SPEC-057 P1: Postgres SKIP LOCKED claim / lease e2e.
//!
//! Run with:
//!   DATABASE_URL=... cargo test -p edgequake-tasks --features postgres --test postgres_claim_lease
//!
//! Skips cleanly when DATABASE_URL / POSTGRES_PASSWORD is unset.

#![cfg(feature = "postgres")]

use std::env;
use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use edgequake_tasks::postgres::PostgresTaskStorage;
use edgequake_tasks::{Task, TaskStatus, TaskStorage, TaskType};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

fn get_database_url() -> Option<String> {
    env::var("DATABASE_URL").ok().or_else(|| {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
        let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
        Some(format!(
            "postgresql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })
}

async fn create_test_pool() -> Option<PgPool> {
    let database_url = get_database_url()?;
    PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url)
        .await
        .ok()
}

/// Ensure mig 088 lease columns exist (idempotent).
async fn ensure_lease_columns(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS lease_owner TEXT")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS lease_token UUID")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ")
        .execute(pool)
        .await?;
    Ok(())
}

/// Make this row the oldest claimable candidate (shared-DB safe).
async fn make_oldest(pool: &PgPool, track_id: &str) -> Result<(), sqlx::Error> {
    let epoch = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
    sqlx::query("UPDATE tasks SET created_at = $2 WHERE track_id = $1")
        .bind(track_id)
        .bind(epoch)
        .execute(pool)
        .await?;
    Ok(())
}

macro_rules! require_postgres {
    () => {
        match create_test_pool().await {
            Some(pool) => {
                if let Err(e) = ensure_lease_columns(&pool).await {
                    eprintln!("Skipping: cannot ensure lease columns: {e}");
                    return;
                }
                if sqlx::query("SELECT 1 FROM tasks LIMIT 0")
                    .execute(&pool)
                    .await
                    .is_err()
                {
                    eprintln!("Skipping: tasks table missing — run migrations first");
                    return;
                }
                pool
            }
            None => {
                eprintln!("Skipping: DATABASE_URL or POSTGRES_PASSWORD not set");
                return;
            }
        }
    };
}

fn sample_task(status: TaskStatus) -> Task {
    let mut task = Task::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        TaskType::Insert,
        serde_json::json!({ "document_id": format!("claim-e2e-{}", Uuid::new_v4()) }),
    );
    task.status = status;
    task
}

/// Seed tenant + workspace so `tasks_*_fkey` constraints pass on shared DBs.
async fn ensure_tenant_workspace(pool: &PgPool, task: &Task) -> Result<(), sqlx::Error> {
    let tenant_slug = format!("claim_t_{}", &task.tenant_id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
        VALUES ($1, $2, $3, TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (tenant_id) DO NOTHING
        "#,
    )
    .bind(task.tenant_id)
    .bind(format!("claim-lease tenant {}", task.tenant_id))
    .bind(&tenant_slug)
    .execute(pool)
    .await?;

    let ws_slug = format!("claim_w_{}", &task.workspace_id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO workspaces (
            workspace_id, tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(task.workspace_id)
    .bind(task.tenant_id)
    .bind(format!("claim-lease workspace {}", task.workspace_id))
    .bind(&ws_slug)
    .execute(pool)
    .await?;

    Ok(())
}

async fn cleanup(pool: &PgPool, task: &Task) {
    let _ = sqlx::query("DELETE FROM tasks WHERE track_id = $1")
        .bind(&task.track_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(task.workspace_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(task.tenant_id)
        .execute(pool)
        .await;
}

async fn release_if_held(storage: &PostgresTaskStorage, task: &Task, worker: &str) {
    if let Some(token) = task.lease_token {
        let _ = storage.release_claim(&task.track_id, worker, token).await;
    }
}

async fn seed_create_oldest(
    pool: &PgPool,
    storage: &PostgresTaskStorage,
    task: &Task,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_tenant_workspace(pool, task).await?;
    storage.create_task(task).await?;
    make_oldest(pool, &task.track_id).await?;
    Ok(())
}

#[tokio::test]
async fn postgres_claim_next_pending_without_wake() {
    let pool = require_postgres!();
    let storage = PostgresTaskStorage::new(pool.clone());
    let task = sample_task(TaskStatus::Pending);
    let track_id = task.track_id.clone();
    seed_create_oldest(&pool, &storage, &task)
        .await
        .expect("seed+create");

    let claimed = storage
        .claim_next("pg-worker-a", Duration::from_secs(120))
        .await
        .expect("claim")
        .expect("Pending must be claimable without channel wake");
    assert_eq!(claimed.track_id, track_id);
    assert_eq!(claimed.status, TaskStatus::Processing);
    assert_eq!(claimed.lease_owner.as_deref(), Some("pg-worker-a"));
    assert!(claimed.lease_token.is_some());

    cleanup(&pool, &task).await;
}

#[tokio::test]
async fn postgres_dual_claim_next_race_one_winner() {
    let pool = require_postgres!();
    let storage = Arc::new(PostgresTaskStorage::new(pool.clone()));
    let task = sample_task(TaskStatus::Pending);
    let track_id = task.track_id.clone();
    seed_create_oldest(&pool, storage.as_ref(), &task)
        .await
        .expect("seed+create");

    let s1 = Arc::clone(&storage);
    let s2 = Arc::clone(&storage);
    let (a, b) = tokio::join!(
        s1.claim_next("race-w1", Duration::from_secs(120)),
        s2.claim_next("race-w2", Duration::from_secs(120)),
    );

    let a = a.expect("claim a");
    let b = b.expect("claim b");
    let a_ours = a.as_ref().map(|t| t.track_id.as_str()) == Some(track_id.as_str());
    let b_ours = b.as_ref().map(|t| t.track_id.as_str()) == Some(track_id.as_str());
    assert!(
        a_ours ^ b_ours,
        "exactly one worker must claim our track; a={a:?} b={b:?}"
    );

    // Release any non-target claim so we don't strand foreign Pending rows.
    if let Some(ref t) = a {
        if !a_ours {
            release_if_held(&s1, t, "race-w1").await;
        }
    }
    if let Some(ref t) = b {
        if !b_ours {
            release_if_held(&s2, t, "race-w2").await;
        }
    }

    cleanup(&pool, &task).await;
}

#[tokio::test]
async fn postgres_cancelled_never_claimed() {
    let pool = require_postgres!();
    let storage = PostgresTaskStorage::new(pool.clone());
    let mut task = sample_task(TaskStatus::Cancelled);
    task.mark_cancelled();
    let track_id = task.track_id.clone();
    seed_create_oldest(&pool, &storage, &task)
        .await
        .expect("seed+create");

    // Oldest is Cancelled — claim must skip it (may return next Pending or None).
    let claimed = storage
        .claim_next("pg-worker-c", Duration::from_secs(120))
        .await
        .expect("claim");
    assert!(
        claimed.as_ref().map(|t| t.track_id.as_str()) != Some(track_id.as_str()),
        "Cancelled must never be claimed"
    );
    if let Some(ref t) = claimed {
        release_if_held(&storage, t, "pg-worker-c").await;
    }

    cleanup(&pool, &task).await;
}

#[tokio::test]
async fn postgres_refresh_lease_and_release_claim_cas() {
    let pool = require_postgres!();
    let storage = PostgresTaskStorage::new(pool.clone());
    let task = sample_task(TaskStatus::Pending);
    let track_id = task.track_id.clone();
    seed_create_oldest(&pool, &storage, &task)
        .await
        .expect("seed+create");

    let claimed = storage
        .claim_next("owner", Duration::from_secs(120))
        .await
        .expect("claim")
        .expect("claimed");
    assert_eq!(claimed.track_id, track_id);
    let token = claimed.lease_token.expect("token");

    assert!(storage
        .refresh_lease(&track_id, "owner", token, Duration::from_secs(120))
        .await
        .expect("refresh"));
    assert!(!storage
        .refresh_lease(&track_id, "intruder", token, Duration::from_secs(120))
        .await
        .expect("refresh wrong owner"));

    assert!(storage
        .release_claim(&track_id, "owner", token)
        .await
        .expect("release"));
    let pending = storage.get_task(&track_id).await.expect("get").unwrap();
    assert_eq!(pending.status, TaskStatus::Pending);
    assert!(pending.lease_owner.is_none());

    cleanup(&pool, &task).await;
}

#[tokio::test]
async fn postgres_claim_reclaims_expired_processing() {
    let pool = require_postgres!();
    let storage = PostgresTaskStorage::new(pool.clone());
    let mut task = sample_task(TaskStatus::Processing);
    task.mark_processing();
    task.lease_owner = Some("dead-worker".into());
    task.lease_token = Some(Uuid::new_v4());
    task.lease_expires_at = Some(Utc::now() - chrono::Duration::seconds(5));
    let track_id = task.track_id.clone();
    seed_create_oldest(&pool, &storage, &task)
        .await
        .expect("seed+create");

    let claimed = storage
        .claim_next("alive-worker", Duration::from_secs(120))
        .await
        .expect("claim")
        .expect("expired Processing must be reclaimable");
    assert_eq!(claimed.track_id, track_id);
    assert_eq!(claimed.lease_owner.as_deref(), Some("alive-worker"));

    cleanup(&pool, &task).await;
}

/// Migration 089 guard: `edgequake.tasks` must expose lease columns (stale-view class).
///
/// `ALTER TABLE public.tasks` alone does not refresh the view; workers with
/// `search_path ("$user", public)` hit the VIEW and fail with missing columns.
#[tokio::test]
async fn edgequake_tasks_view_exposes_lease_columns() {
    let pool = require_postgres!();

    let names: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT column_name::text
        FROM information_schema.columns
        WHERE table_schema = 'edgequake'
          AND table_name = 'tasks'
          AND column_name IN ('lease_owner', 'lease_token', 'lease_expires_at')
        ORDER BY column_name
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("query edgequake.tasks columns");

    assert_eq!(
        names,
        vec![
            "lease_expires_at".to_string(),
            "lease_owner".to_string(),
            "lease_token".to_string(),
        ],
        "edgequake.tasks must expose lease_* after migration 089 (got {names:?})"
    );

    // Prove SELECT through the view (same relation workers hit under edgequake search_path).
    sqlx::query("SELECT lease_owner, lease_token, lease_expires_at FROM edgequake.tasks LIMIT 0")
        .execute(&pool)
        .await
        .expect("SELECT lease_* via edgequake.tasks must succeed");
}

/// SPEC-084 / GH-316: after WS-A holds an active lease, WS-B interleaves.
#[tokio::test]
async fn issue316_two_workspaces_interleaved_progress() {
    let pool = require_postgres!();
    let storage = PostgresTaskStorage::new(pool.clone());

    let tenant = Uuid::new_v4();
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();

    let mut tasks_a = Vec::new();
    for i in 0..4 {
        let mut t = Task::new(
            tenant,
            ws_a,
            TaskType::Insert,
            serde_json::json!({ "document_id": format!("issue316-a-{i}") }),
        );
        t.status = TaskStatus::Pending;
        ensure_tenant_workspace(&pool, &t).await.expect("seed tw");
        storage.create_task(&t).await.expect("create a");
        // Shared-DB: pin far in the past so our rows win claim ordering.
        let ts = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, i as u32).unwrap();
        sqlx::query("UPDATE tasks SET created_at = $2 WHERE track_id = $1")
            .bind(&t.track_id)
            .bind(ts)
            .execute(&pool)
            .await
            .expect("pin a");
        tasks_a.push(t);
    }

    let mut task_b = Task::new(
        tenant,
        ws_b,
        TaskType::Insert,
        serde_json::json!({ "document_id": "issue316-b-0" }),
    );
    task_b.status = TaskStatus::Pending;
    ensure_tenant_workspace(&pool, &task_b)
        .await
        .expect("seed b tw");
    storage.create_task(&task_b).await.expect("create b");
    let ts_b = Utc.with_ymd_and_hms(1970, 1, 1, 0, 1, 0).unwrap();
    sqlx::query("UPDATE tasks SET created_at = $2 WHERE track_id = $1")
        .bind(&task_b.track_id)
        .bind(ts_b)
        .execute(&pool)
        .await
        .expect("pin b");

    let first = storage
        .claim_next("issue316-w1", Duration::from_secs(120))
        .await
        .expect("claim1")
        .expect("must claim");
    assert_eq!(first.workspace_id, ws_a, "oldest backlog workspace first");

    let second = storage
        .claim_next("issue316-w2", Duration::from_secs(120))
        .await
        .expect("claim2")
        .expect("must claim");
    assert_eq!(
        second.workspace_id, ws_b,
        "zero-active workspace must interleave before A backlog drains"
    );

    release_if_held(&storage, &first, "issue316-w1").await;
    release_if_held(&storage, &second, "issue316-w2").await;
    for t in &tasks_a {
        cleanup(&pool, t).await;
    }
    cleanup(&pool, &task_b).await;
}

/// SPEC-084 / GH-316: tenant ingest cap still binds across workspaces.
#[tokio::test]
async fn issue316_tenant_cap_still_holds() {
    use edgequake_tasks::{FairnessClass, TenantConcurrencyLimiter, TryAcquireOutcome};

    let limiter = TenantConcurrencyLimiter::new(2, 2);
    let tenant = Uuid::new_v4();
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();
    let ws_c = Uuid::new_v4();

    let _a = match limiter
        .try_acquire(tenant, ws_a, FairnessClass::Ingest)
        .await
    {
        TryAcquireOutcome::Acquired(p) => p,
        other => panic!("expected Acquired, got {other:?}"),
    };
    let _b = match limiter
        .try_acquire(tenant, ws_b, FairnessClass::Ingest)
        .await
    {
        TryAcquireOutcome::Acquired(p) => p,
        other => panic!("expected Acquired, got {other:?}"),
    };
    assert!(
        matches!(
            limiter
                .try_acquire(tenant, ws_c, FairnessClass::Ingest)
                .await,
            TryAcquireOutcome::AtCapacity
        ),
        "tenant ingest cap must still hold with workspace lanes"
    );
}
