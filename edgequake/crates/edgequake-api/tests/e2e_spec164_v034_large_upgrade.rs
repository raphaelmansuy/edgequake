//! Upgrade a v0.34.0 schema (migration 169, published checksum of 167) that
//! already holds a full-size corpus. The migrator must accept that ledger,
//! apply 170–207, keep every row, and leave the new status index able to
//! stop at the page.

#![cfg(feature = "postgres")]

use std::process::Command;
use std::time::Instant;

use edgequake_api::state::migration_bootstrap::{
    run_postgres_migrations, run_postgres_migrations_through,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use uuid::Uuid;

const DOCUMENTS: i64 = 80_000;
const CHUNKS: i64 = 40_000;
const CONVERSATIONS: i64 = 30_000;
const TASKS: i64 = 80_000;

#[tokio::test]
async fn v034_large_database_upgrades_without_losing_rows() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP v034 large upgrade: set DATABASE_URL");
        return;
    };
    let published = published_167_sha384();
    let db = format!("spec164up_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let url = provision(&admin, &db).await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(60))
        .connect(&url)
        .await
        .expect("db connect");

    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let to_published = Instant::now();
    run_postgres_migrations_through(&pool, 169)
        .await
        .expect("apply the v0.34.0 train through 169");
    let published_ms = to_published.elapsed().as_secs_f64() * 1000.0;
    let at_169: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM public._sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("max at 169");
    assert_eq!(at_169, 169, "the published schema stops at 169");
    assert_public_ledger(&pool, &[15, 16, 169]).await;

    sqlx::query(
        "UPDATE public._sqlx_migrations SET checksum = decode($1, 'hex') WHERE version = 167",
    )
    .bind(&published)
    .execute(&pool)
    .await
    .expect("install the v0.34.0 checksum for migration 167");

    let (tenant, workspace, user) = load_corpus(&pool).await;
    let docs_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.documents WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .expect("document count");
    assert_eq!(docs_before, DOCUMENTS);

    let upgrade = Instant::now();
    run_postgres_migrations(&pool)
        .await
        .expect("upgrade the large v0.34.0 database through the current train");
    let upgrade_ms = upgrade.elapsed().as_secs_f64() * 1000.0;

    let head: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM public._sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("head");
    assert_eq!(head, 207, "the large database must reach migration 207");
    assert_public_ledger(&pool, &[15, 16, 169, 202]).await;
    let stored_167: String = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex') FROM public._sqlx_migrations WHERE version = 167",
    )
    .fetch_one(&pool)
    .await
    .expect("167 checksum");
    assert_ne!(
        stored_167, published,
        "the published 167 checksum must be rewritten to the current file"
    );
    let docs_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.documents WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .expect("documents after");
    let chunks_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.chunks WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .expect("chunks after");
    let conversations_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.conversations WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .expect("conversations after");
    let tasks_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.tasks WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .expect("tasks after");
    assert_eq!(docs_after, DOCUMENTS);
    assert_eq!(chunks_after, CHUNKS);
    assert_eq!(conversations_after, CONVERSATIONS);
    assert_eq!(tasks_after, TASKS);
    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.documents WHERE workspace_id = $1 AND status = 'failed'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("failed count");
    assert_eq!(failed, 50);

    let age_on: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age");
    assert!(!age_on, "migration 171 drops Apache AGE");

    sqlx::query("ANALYZE public.documents")
        .execute(&pool)
        .await
        .expect("analyze");
    let plan: serde_json::Value = sqlx::query(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
         SELECT id FROM public.documents
         WHERE workspace_id = $1 AND status = 'failed'
         ORDER BY created_at DESC, id DESC
         LIMIT 50",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("explain")
    .get::<serde_json::Value, _>("QUERY PLAN");
    let plan_text = plan.to_string();
    assert!(
        !plan_text.contains("Seq Scan"),
        "the failed page must not read every document, plan={plan_text}"
    );
    let execution_ms = plan[0]["Execution Time"].as_f64().unwrap_or(1_000.0);
    assert!(
        execution_ms < 50.0,
        "status page scanned too much: {execution_ms} ms, plan={plan_text}"
    );
    // Fifty failed rows in one workspace make the narrower status index the
    // cheaper stop. Hide it inside a rolled-back transaction so this corpus
    // also proves migration 200's workspace index stops at the page.
    let mut conn = pool.acquire().await.expect("explain conn");
    sqlx::query("BEGIN")
        .execute(&mut *conn)
        .await
        .expect("begin");
    let hidden = sqlx::query(
        "UPDATE pg_index SET indisvalid = false \
         WHERE indrelid = 'public.documents'::regclass \
           AND indexrelid <> 'public.idx_documents_workspace_status_created'::regclass",
    )
    .execute(&mut *conn)
    .await
    .expect("hide the narrower status index");
    assert!(hidden.rows_affected() >= 1);
    let forced: serde_json::Value = sqlx::query(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
         SELECT id FROM public.documents
         WHERE workspace_id = $1 AND status = 'failed'
         ORDER BY created_at DESC, id DESC
         LIMIT 50",
    )
    .bind(workspace)
    .fetch_one(&mut *conn)
    .await
    .expect("explain composite")
    .get::<serde_json::Value, _>("QUERY PLAN");
    sqlx::query("ROLLBACK")
        .execute(&mut *conn)
        .await
        .expect("restore the status index");
    let forced_text = forced.to_string();
    assert!(
        forced_text.contains("idx_documents_workspace_status_created"),
        "migration 200's index must stop the failed page, plan={forced_text}"
    );
    let forced_ms = forced[0]["Execution Time"].as_f64().unwrap_or(1_000.0);
    assert!(
        forced_ms < 50.0,
        "workspace status index scanned too much: {forced_ms} ms, plan={forced_text}"
    );

    let body = serde_json::json!({
        "published_schema": "v0.34.0",
        "published_head": 169,
        "published_167_sha384": published,
        "upgraded_head": 202,
        "apply_through_169_ms": published_ms,
        "upgrade_170_through_202_ms": upgrade_ms,
        "documents": docs_after,
        "chunks": chunks_after,
        "conversations": conversations_after,
        "tasks": tasks_after,
        "failed_page_ms": execution_ms,
        "failed_page_on_workspace_index_ms": forced_ms,
        "tenant_id": tenant,
        "workspace_id": workspace,
        "user_id": user,
    });
    std::fs::write(
        "/tmp/spec164-v034-large-upgrade.json",
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .expect("write measurements");
    eprintln!("{}", serde_json::to_string_pretty(&body).unwrap());

    pool.close().await;
    drop_db(&admin, &db).await;
}

#[tokio::test]
async fn v034_train_keeps_versions_15_and_16_in_public() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP v034 ledger: set DATABASE_URL");
        return;
    };
    let db = format!("spec164up_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let url = provision(&admin, &db).await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(60))
        .connect(&url)
        .await
        .expect("db connect");
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    run_postgres_migrations_through(&pool, 169)
        .await
        .expect("published train");
    assert_public_ledger(&pool, &[15, 16, 169]).await;

    sqlx::query(
        "CREATE TABLE edgequake._sqlx_migrations (LIKE public._sqlx_migrations INCLUDING ALL)",
    )
    .execute(&pool)
    .await
    .expect("stray ledger");
    sqlx::query(
        "INSERT INTO edgequake._sqlx_migrations
         SELECT * FROM public._sqlx_migrations WHERE version = 16",
    )
    .execute(&pool)
    .await
    .expect("copy version 16");
    sqlx::query("DELETE FROM public._sqlx_migrations WHERE version = 16")
        .execute(&pool)
        .await
        .expect("hide version 16 from public");

    run_postgres_migrations_through(&pool, 169)
        .await
        .expect("fold the stray ledger");
    assert_public_ledger(&pool, &[15, 16, 169]).await;
    let copies: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public._sqlx_migrations WHERE version = 16")
            .fetch_one(&pool)
            .await
            .expect("one version 16");
    assert_eq!(copies, 1, "folding must not duplicate version 16");

    pool.close().await;
    drop_db(&admin, &db).await;
}

async fn assert_public_ledger(pool: &sqlx::PgPool, required: &[i64]) {
    let stray: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE c.relname = '_sqlx_migrations'
           AND c.relkind = 'r'
           AND n.nspname <> 'public'",
    )
    .fetch_one(pool)
    .await
    .expect("stray ledgers");
    assert_eq!(stray, 0, "the ledger must live only in public");
    for version in required {
        let present: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM public._sqlx_migrations
                WHERE version = $1 AND success = true
             )",
        )
        .bind(version)
        .fetch_one(pool)
        .await
        .expect("version present");
        assert!(
            present,
            "public._sqlx_migrations is missing version {version}"
        );
    }
}

fn published_167_sha384() -> String {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("git")
        .args([
            "show",
            "v0.34.0:edgequake/migrations/167_tenant_access_rls.sql",
        ])
        .current_dir(&repo)
        .output()
        .expect("git show the tagged migration");
    assert!(
        output.status.success(),
        "git show v0.34.0 migration 167 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let digest = Command::new("openssl")
        .args(["dgst", "-sha384"])
        .current_dir(&repo)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("openssl stdin")
                .write_all(&output.stdout)?;
            let done = child.wait_with_output()?;
            Ok(done)
        })
        .expect("openssl sha384");
    assert!(digest.status.success());
    let text = String::from_utf8(digest.stdout).expect("utf8");
    let hash = text
        .split('=')
        .next_back()
        .expect("digest")
        .trim()
        .to_string();
    assert_eq!(
        hash, "a014ecdc029cf05cc500703f8ca9a961c3de8d92326c4094911dd70561cac4399007871855ce47187f191bfbcf543d1b",
        "the fossil must be the bytes tagged v0.34.0"
    );
    hash
}

async fn load_corpus(pool: &sqlx::PgPool) -> (Uuid, Uuid, Uuid) {
    let (tenant, workspace, user) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut conn = pool.acquire().await.expect("conn");
    sqlx::query("SET statement_timeout = 0")
        .execute(&mut *conn)
        .await
        .expect("timeout");
    sqlx::query("INSERT INTO public.tenants (tenant_id, name, slug) VALUES ($1, 'upgrade', $2)")
        .bind(tenant)
        .bind(format!("t{}", &tenant.simple().to_string()[..12]))
        .execute(&mut *conn)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO public.workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, 'upgrade', $3)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(format!("w{}", &workspace.simple().to_string()[..12]))
    .execute(&mut *conn)
    .await
    .expect("workspace");
    sqlx::query(
        "INSERT INTO public.users (user_id, tenant_id, username, email) VALUES ($1, $2, $3, $4)",
    )
    .bind(user)
    .bind(tenant)
    .bind(format!("u{}", &user.simple().to_string()[..12]))
    .bind(format!("{user}@example.com"))
    .execute(&mut *conn)
    .await
    .expect("user");

    sqlx::query(
        "INSERT INTO public.documents (tenant_id, workspace_id, title, content, status, created_at, updated_at)
         SELECT $1, $2, 'd' || g, '', 'indexed',
                timestamptz '2024-01-01' + (g * interval '1 second'),
                timestamptz '2024-01-01' + (g * interval '1 second')
         FROM generate_series(1, $3) AS g",
    )
    .bind(tenant)
    .bind(workspace)
    .bind(DOCUMENTS)
    .execute(&mut *conn)
    .await
    .expect("documents");
    sqlx::query(
        "UPDATE public.documents SET status = 'failed'
         WHERE id IN (
           SELECT id FROM public.documents
           WHERE workspace_id = $1
           ORDER BY created_at ASC, id ASC
           LIMIT 50
         )",
    )
    .bind(workspace)
    .execute(&mut *conn)
    .await
    .expect("failed rows");
    sqlx::query(
        "INSERT INTO public.chunks (document_id, tenant_id, workspace_id, chunk_index, content)
         SELECT id, tenant_id, workspace_id, 0, 'chunk'
         FROM (
           SELECT id, tenant_id, workspace_id
           FROM public.documents
           WHERE workspace_id = $1
           ORDER BY created_at ASC
           LIMIT $2
         ) page",
    )
    .bind(workspace)
    .bind(CHUNKS)
    .execute(&mut *conn)
    .await
    .expect("chunks");
    sqlx::query(
        "INSERT INTO public.conversations (tenant_id, workspace_id, user_id, title, created_at, updated_at)
         SELECT $1, $2, $3, 'c' || g,
                timestamptz '2024-01-01' + (g * interval '1 second'),
                timestamptz '2024-01-01' + (g * interval '1 second')
         FROM generate_series(1, $4) AS g",
    )
    .bind(tenant)
    .bind(workspace)
    .bind(user)
    .bind(CONVERSATIONS)
    .execute(&mut *conn)
    .await
    .expect("conversations");
    sqlx::query(
        "INSERT INTO public.tasks (tenant_id, workspace_id, track_id, task_type, status, created_at, updated_at)
         SELECT $1, $2, 'task-' || g, 'insert', 'pending',
                timestamptz '2024-01-01' + (g * interval '1 second'),
                timestamptz '2024-01-01' + (g * interval '1 second')
         FROM generate_series(1, $3) AS g",
    )
    .bind(tenant)
    .bind(workspace)
    .bind(TASKS)
    .execute(&mut *conn)
    .await
    .expect("tasks");
    (tenant, workspace, user)
}

async fn provision(admin: &str, db: &str) -> String {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin)
        .await
        .expect("admin");
    sqlx::query(&format!("CREATE DATABASE {db}"))
        .execute(&admin_pool)
        .await
        .expect("create database");
    admin_pool.close().await;
    replace_db(admin, db)
}

async fn drop_db(admin: &str, db: &str) {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin)
        .await
        .expect("admin drop");
    sqlx::query(&format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db}' AND pid <> pg_backend_pid()"
    ))
    .execute(&admin_pool)
    .await
    .ok();
    sqlx::query(&format!("DROP DATABASE IF EXISTS {db}"))
        .execute(&admin_pool)
        .await
        .ok();
}

fn admin_url() -> Option<String> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::fs::read_to_string("/tmp/edgequake-db-url").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if url.is_none() && std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS").as_deref() == Ok("1") {
        panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but DATABASE_URL is unset");
    }
    url
}

fn replace_db(url: &str, db: &str) -> String {
    let q = url.find('?').map(|i| &url[i..]).unwrap_or("");
    let without_q = url.trim_end_matches(q);
    let slash = without_q.rfind('/').expect("db slash");
    format!("{}{}{}", &without_q[..slash + 1], db, q)
}
