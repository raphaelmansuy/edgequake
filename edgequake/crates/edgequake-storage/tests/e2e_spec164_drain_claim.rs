//! Scoped drain claims must keep the caller's workspace and skip older rows
//! that belong to someone else.
//!
//! Run:
//!   DATABASE_URL=postgres://edgequake:edgequake@127.0.0.1/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec164_drain_claim
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use std::sync::{Arc, Mutex};

use edgequake_storage::compensation_drain::{self, DrainConfig, DrainMode};
use edgequake_storage::{outbox_drain_once, OutboxDrainConfig};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use uuid::Uuid;

async fn column_exists(pool: &sqlx::PgPool, table: &str, column: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2
        )",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn drop_workspace(pool: &sqlx::PgPool, workspace: Uuid) {
    let tenant = w3::workspace_tenant(pool, workspace).await;
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(pool)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(pool)
        .await
        .expect("cleanup tenant");
}

#[tokio::test(flavor = "current_thread")]
async fn scoped_outbox_claim_skips_the_other_workspaces_older_events() {
    let Some(cfg) = require_or_skip_postgres("spec164_outbox_claim") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    if !column_exists(&pool, "outbox_events", "available_at").await {
        eprintln!("skip: outbox available_at missing");
        return;
    }

    let older = w3::seed_workspace(&pool, "spec164-outbox-older").await;
    let caller = w3::seed_workspace(&pool, "spec164-outbox-caller").await;
    let older_id = Uuid::new_v4();
    let caller_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbox_events \
         (id, aggregate_type, aggregate_id, event_type, workspace_id, available_at, created_at) \
         VALUES ($1, 'chunk', $1, 'chunk_ready', $2, now() - interval '2 hours', now() - interval '2 hours')",
    )
    .bind(older_id)
    .bind(older)
    .execute(&pool)
    .await
    .expect("older outbox");
    sqlx::query(
        "INSERT INTO outbox_events \
         (id, aggregate_type, aggregate_id, event_type, workspace_id, available_at, created_at) \
         VALUES ($1, 'chunk', $1, 'chunk_ready', $2, now() - interval '1 minute', now() - interval '1 minute')",
    )
    .bind(caller_id)
    .bind(caller)
    .execute(&pool)
    .await
    .expect("caller outbox");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    let mut scoped = OutboxDrainConfig::from_env().with_workspace_scope(caller);
    scoped.batch = 10;
    outbox_drain_once(&pool, &scoped, &move |event| {
        let record = Arc::clone(&record);
        async move {
            record.lock().expect("seen").push(event.id);
            Ok(())
        }
    })
    .await
    .expect("scoped outbox claim");

    let open_seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&open_seen);
    let mut open = OutboxDrainConfig::from_env();
    open.batch = 1;
    outbox_drain_once(&pool, &open, &move |event| {
        let record = Arc::clone(&record);
        async move {
            record.lock().expect("seen").push(event.id);
            Ok(())
        }
    })
    .await
    .expect("open outbox claim");

    sqlx::query("DELETE FROM outbox_events WHERE id = $1 OR id = $2")
        .bind(older_id)
        .bind(caller_id)
        .execute(&pool)
        .await
        .expect("cleanup outbox");
    drop_workspace(&pool, older).await;
    drop_workspace(&pool, caller).await;

    let scoped_ids = seen.lock().expect("seen").clone();
    assert_eq!(
        scoped_ids,
        vec![caller_id],
        "scoped claim must keep only the caller's event"
    );
    let open_ids = open_seen.lock().expect("seen").clone();
    assert_eq!(
        open_ids,
        vec![older_id],
        "a drain with no workspace must take the older event"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn scoped_quarantine_claim_skips_the_other_workspaces_older_rows() {
    let Some(cfg) = require_or_skip_postgres("spec164_quarantine_claim") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    if !column_exists(&pool, "compensation_quarantine", "next_attempt_at").await {
        eprintln!("skip: compensation_quarantine missing");
        return;
    }

    let older = w3::seed_workspace(&pool, "spec164-quarantine-older").await;
    let caller = w3::seed_workspace(&pool, "spec164-quarantine-caller").await;
    let doc = w3::seed_document(&pool, older).await;
    let older_id = Uuid::new_v4();
    let caller_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO compensation_quarantine \
         (entry_id, document_id, workspace_id, status, next_attempt_at) \
         VALUES ($1, $2, $3, 'pending', now() - interval '2 hours')",
    )
    .bind(older_id)
    .bind(doc)
    .bind(older)
    .execute(&pool)
    .await
    .expect("older quarantine");
    sqlx::query(
        "INSERT INTO compensation_quarantine \
         (entry_id, document_id, workspace_id, status, next_attempt_at) \
         VALUES ($1, $2, $3, 'failed', now() - interval '1 minute')",
    )
    .bind(caller_id)
    .bind(doc)
    .bind(caller)
    .execute(&pool)
    .await
    .expect("caller quarantine");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    let mut scoped = DrainConfig::from_env().with_workspace_scope(caller);
    scoped.mode = DrainMode::On;
    scoped.batch = 10;
    compensation_drain::drain_once(&pool, &scoped, &move |entry| {
        let record = Arc::clone(&record);
        async move {
            record.lock().expect("seen").push(entry.entry_id);
            Ok(())
        }
    })
    .await
    .expect("scoped quarantine claim");

    let open_seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&open_seen);
    let mut open = DrainConfig::from_env();
    open.mode = DrainMode::On;
    open.batch = 1;
    compensation_drain::drain_once(&pool, &open, &move |entry| {
        let record = Arc::clone(&record);
        async move {
            record.lock().expect("seen").push(entry.entry_id);
            Ok(())
        }
    })
    .await
    .expect("open quarantine claim");

    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc)
        .execute(&pool)
        .await
        .expect("cleanup document");
    drop_workspace(&pool, older).await;
    drop_workspace(&pool, caller).await;

    let scoped_ids = seen.lock().expect("seen").clone();
    assert_eq!(
        scoped_ids,
        vec![caller_id],
        "scoped claim must keep the caller's failed row and skip the older workspace"
    );
    let open_ids = open_seen.lock().expect("seen").clone();
    assert_eq!(
        open_ids,
        vec![older_id],
        "a drain with no workspace must take the older pending row"
    );
}
