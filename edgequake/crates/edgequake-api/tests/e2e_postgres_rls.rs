//! PostgreSQL Row Level Security (RLS) E2E Tests
//!
//! These tests verify tenant isolation at the database level using PostgreSQL RLS.
//! Run with: cargo test --package edgequake-api --test e2e_postgres_rls
//!
//! SPEC-091 IW0 (GAP-091-27): the suite is no longer `#[ignore]`d — it runs by
//! default and soft-skips (with a notice) when the RLS rig is unreachable, so
//! plain `cargo test` stays green locally while CI (`postgres-tests` job,
//! app_user + admin URLs) enforces it as a required gate.
//!
//! Key Insights:
//! 1. Superusers ALWAYS bypass RLS - we use app_user (non-superuser) for testing
//! 2. set_config is session-scoped - we must use the same connection for set and query
//! 3. Connection pools give different connections - use acquire() for dedicated connection

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use uuid::Uuid;

/// Probe both RLS rig pools once; soft-skip (None) when unreachable so the
/// default test run degrades gracefully outside the CI rig. SPEC-091 IW0.
async fn require_rls_rig() -> Option<(Pool<Postgres>, Pool<Postgres>)> {
    match (create_admin_pool().await, create_test_pool().await) {
        (Ok(admin_pool), Ok(test_pool)) => Some((admin_pool, test_pool)),
        (admin_result, test_result) => {
            assert_ne!(
                std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
                    .ok()
                    .as_deref(),
                Some("1"),
                "Required RLS rig unavailable: admin={}, app_user={}",
                admin_result.is_ok(),
                test_result.is_ok()
            );
            eprintln!(
                "SKIP e2e_postgres_rls: RLS rig unreachable (admin: {}, app_user: {}) — \
                 set ADMIN_DATABASE_URL/TEST_DATABASE_URL (CI postgres-tests job provides them)",
                admin_result.is_ok(),
                test_result.is_ok()
            );
            None
        }
    }
}

/// Create non-superuser pool for RLS testing
async fn create_test_pool() -> Result<Pool<Postgres>, sqlx::Error> {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://app_user:app_password_123@localhost:5433/edgequake_test".to_string()
    });

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

/// Create superuser pool for admin operations
async fn create_admin_pool() -> Result<Pool<Postgres>, sqlx::Error> {
    let database_url = std::env::var("ADMIN_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test".to_string()
    });

    PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
}

/// Ensure a tenant row exists (documents.tenant_id FK).
async fn ensure_tenant(admin_pool: &Pool<Postgres>, tenant_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, name, slug)
        VALUES ($1, $2, $3)
        ON CONFLICT (tenant_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(format!("tenant-{tenant_id}"))
    .bind(format!("t-{tenant_id}"))
    .execute(admin_pool)
    .await?;
    Ok(())
}

/// Ensure workspace exists under a tenant (document_originals FK).
async fn ensure_workspace(
    admin_pool: &Pool<Postgres>,
    workspace_id: Uuid,
    tenant_id: Uuid,
) -> Result<(), sqlx::Error> {
    ensure_tenant(admin_pool, tenant_id).await?;
    sqlx::query(
        r#"
        INSERT INTO workspaces (workspace_id, tenant_id, name, slug)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("ws-{workspace_id}"))
    .bind(format!("w-{workspace_id}"))
    .execute(admin_pool)
    .await?;
    Ok(())
}

/// Admin insert that seeds the tenant FK first.
async fn admin_insert_document(
    admin_pool: &Pool<Postgres>,
    doc_id: Uuid,
    tenant_id: Uuid,
    title: &str,
    content: &str,
) -> Result<(), sqlx::Error> {
    ensure_tenant(admin_pool, tenant_id).await?;
    sqlx::query("INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, $3, $4)")
        .bind(doc_id)
        .bind(tenant_id)
        .bind(title)
        .bind(content)
        .execute(admin_pool)
        .await?;
    Ok(())
}

/// SPEC-083 S-03: set GUC with `is_local=true` inside an explicit transaction
/// so the tenant context is visible to following statements and rolls back
/// cleanly (never autocommit session GUC).
async fn begin_tenant_tx(
    conn: &mut sqlx::pool::PoolConnection<Postgres>,
    tenant_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("BEGIN").execute(&mut **conn).await?;
    sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut **conn)
        .await?;
    Ok(())
}

async fn commit_tx(conn: &mut sqlx::pool::PoolConnection<Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query("COMMIT").execute(&mut **conn).await?;
    Ok(())
}

async fn rollback_tx(conn: &mut sqlx::pool::PoolConnection<Postgres>) -> Result<(), sqlx::Error> {
    let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
    Ok(())
}

/// Query with tenant context on a dedicated connection
/// This ensures set_config and query use the same transaction
#[allow(dead_code)]
async fn query_with_tenant_context<T>(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    query: &str,
) -> Result<T, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let mut conn = pool.acquire().await?;
    begin_tenant_tx(&mut conn, tenant_id).await?;
    let result = sqlx::query_as::<_, T>(query).fetch_one(&mut *conn).await;
    match result {
        Ok(row) => {
            commit_tx(&mut conn).await?;
            Ok(row)
        }
        Err(e) => {
            rollback_tx(&mut conn).await?;
            Err(e)
        }
    }
}

/// Count documents with tenant context
async fn count_documents_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    begin_tenant_tx(&mut conn, tenant_id).await?;
    let count: Result<(i64,), _> = sqlx::query_as("SELECT COUNT(*) FROM documents")
        .fetch_one(&mut *conn)
        .await;
    match count {
        Ok(c) => {
            commit_tx(&mut conn).await?;
            Ok(c.0)
        }
        Err(e) => {
            rollback_tx(&mut conn).await?;
            Err(e)
        }
    }
}

/// Execute UPDATE with tenant context, returns rows affected
async fn update_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    query: &str,
    bind_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    begin_tenant_tx(&mut conn, tenant_id).await?;
    let result = sqlx::query(query).bind(bind_id).execute(&mut *conn).await;
    match result {
        Ok(r) => {
            commit_tx(&mut conn).await?;
            Ok(r.rows_affected())
        }
        Err(e) => {
            rollback_tx(&mut conn).await?;
            Err(e)
        }
    }
}

/// Execute DELETE with tenant context, returns rows affected
async fn delete_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    doc_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    begin_tenant_tx(&mut conn, tenant_id).await?;
    let result = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(&mut *conn)
        .await;
    match result {
        Ok(r) => {
            commit_tx(&mut conn).await?;
            Ok(r.rows_affected())
        }
        Err(e) => {
            rollback_tx(&mut conn).await?;
            Err(e)
        }
    }
}

/// Query document by ID with tenant context
async fn get_document_title_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    doc_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    begin_tenant_tx(&mut conn, tenant_id).await?;
    let result: Result<Option<(String,)>, _> =
        sqlx::query_as("SELECT title FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_optional(&mut *conn)
            .await;
    match result {
        Ok(row) => {
            commit_tx(&mut conn).await?;
            Ok(row.map(|r| r.0))
        }
        Err(e) => {
            rollback_tx(&mut conn).await?;
            Err(e)
        }
    }
}

/// Insert document with tenant context
async fn insert_as_tenant(
    pool: &Pool<Postgres>,
    admin_pool: &Pool<Postgres>,
    tenant_id: Uuid,
    doc_id: Uuid,
    insert_tenant_id: Uuid,
    title: &str,
) -> Result<(), sqlx::Error> {
    // Seed FK targets via admin (app_user cannot insert into tenants).
    ensure_tenant(admin_pool, tenant_id).await?;
    ensure_tenant(admin_pool, insert_tenant_id).await?;
    let mut conn = pool.acquire().await?;
    begin_tenant_tx(&mut conn, tenant_id).await?;
    let result = sqlx::query(
        "INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, $3, 'Content')",
    )
    .bind(doc_id)
    .bind(insert_tenant_id)
    .bind(title)
    .execute(&mut *conn)
    .await;
    match result {
        Ok(_) => {
            commit_tx(&mut conn).await?;
            Ok(())
        }
        Err(e) => {
            rollback_tx(&mut conn).await?;
            Err(e)
        }
    }
}

#[tokio::test]
async fn test_postgres_rls_basic_isolation() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_a = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    // Insert using admin pool (bypasses RLS)
    admin_insert_document(&admin_pool, doc_a, tenant_a, "Doc A", "Content A")
        .await
        .expect("Failed to insert doc A");
    admin_insert_document(&admin_pool, doc_b, tenant_b, "Doc B", "Content B")
        .await
        .expect("Failed to insert doc B");

    // Test: As tenant A, should only see 1 document
    let count_a = count_documents_as_tenant(&test_pool, tenant_a)
        .await
        .expect("Failed to count as tenant A");

    assert_eq!(
        count_a, 1,
        "Tenant A should see exactly 1 document with RLS"
    );

    // Test: As tenant B, should only see 1 document
    let count_b = count_documents_as_tenant(&test_pool, tenant_b)
        .await
        .expect("Failed to count as tenant B");

    assert_eq!(
        count_b, 1,
        "Tenant B should see exactly 1 document with RLS"
    );
}

#[tokio::test]
async fn test_postgres_rls_cross_tenant_query_blocked() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    admin_insert_document(&admin_pool, doc_b, tenant_b, "Secret B", "Confidential")
        .await
        .expect("Failed to insert doc B");
    ensure_tenant(&admin_pool, tenant_a)
        .await
        .expect("ensure tenant A");

    // Test: Tenant A tries to access Tenant B's document by ID
    let result = get_document_title_as_tenant(&test_pool, tenant_a, doc_b)
        .await
        .expect("Query failed");

    assert!(
        result.is_none(),
        "RLS should block tenant A from seeing tenant B's document"
    );

    // Verify tenant B can see their own document
    let result_b = get_document_title_as_tenant(&test_pool, tenant_b, doc_b)
        .await
        .expect("Query failed");

    assert!(result_b.is_some(), "Tenant B should see their own document");
    assert_eq!(result_b.unwrap(), "Secret B");
}

#[tokio::test]
async fn test_postgres_update_isolation() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    admin_insert_document(&admin_pool, doc_b, tenant_b, "Original B", "Content")
        .await
        .expect("Failed to insert doc B");
    ensure_tenant(&admin_pool, tenant_a)
        .await
        .expect("ensure tenant A");

    // Test: Tenant A tries to update Tenant B's document
    let rows_affected = update_as_tenant(
        &test_pool,
        tenant_a,
        "UPDATE documents SET title = 'Hacked!' WHERE id = $1",
        doc_b,
    )
    .await
    .expect("Update query failed");

    assert_eq!(
        rows_affected, 0,
        "RLS should block tenant A from updating tenant B's document"
    );

    // Verify document is unchanged (check with admin)
    let title: (String,) = sqlx::query_as("SELECT title FROM documents WHERE id = $1")
        .bind(doc_b)
        .fetch_one(&admin_pool)
        .await
        .expect("Failed to fetch doc B");

    assert_eq!(title.0, "Original B", "Document B should be unchanged");
}

#[tokio::test]
async fn test_postgres_delete_isolation() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    admin_insert_document(&admin_pool, doc_b, tenant_b, "Keep B", "Content")
        .await
        .expect("Failed to insert doc B");
    ensure_tenant(&admin_pool, tenant_a)
        .await
        .expect("ensure tenant A");

    // Test: Tenant A tries to delete Tenant B's document
    let rows_affected = delete_as_tenant(&test_pool, tenant_a, doc_b)
        .await
        .expect("Delete query failed");

    assert_eq!(
        rows_affected, 0,
        "RLS should block tenant A from deleting tenant B's document"
    );

    // Verify document still exists (check with admin)
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1)")
        .bind(doc_b)
        .fetch_one(&admin_pool)
        .await
        .expect("Failed to check existence");

    assert!(
        exists.0,
        "Document B should still exist after failed delete"
    );
}

#[tokio::test]
async fn test_rls_insert_isolation() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_id = Uuid::new_v4();

    // Test: Tenant A tries to insert a document with Tenant B's ID
    let result = insert_as_tenant(
        &test_pool,
        &admin_pool,
        tenant_a,
        doc_id,
        tenant_b,
        "Sneaky",
    )
    .await;

    assert!(
        result.is_err(),
        "RLS WITH CHECK should prevent inserting documents with wrong tenant_id"
    );

    // Verify no document was inserted (check with admin)
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE id = $1")
        .bind(doc_id)
        .fetch_one(&admin_pool)
        .await
        .expect("Failed to count");

    assert_eq!(
        count.0, 0,
        "No document should have been inserted with wrong tenant_id"
    );

    // Test: Tenant A can insert with their own tenant_id
    let doc_id_valid = Uuid::new_v4();
    let result_valid = insert_as_tenant(
        &test_pool,
        &admin_pool,
        tenant_a,
        doc_id_valid,
        tenant_a,
        "Valid",
    )
    .await;

    assert!(
        result_valid.is_ok(),
        "Tenant A should be able to insert with their own tenant_id"
    );
}

#[tokio::test]
async fn test_tenant_isolation_with_concurrent_access() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    // Create 5 tenants with 3 documents each
    let tenants: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    // Insert using admin pool
    for (i, tenant) in tenants.iter().enumerate() {
        for j in 0..3 {
            let doc_id = Uuid::new_v4();
            admin_insert_document(
                &admin_pool,
                doc_id,
                *tenant,
                &format!("Doc {} for Tenant {}", j, i),
                &format!("Content {} for Tenant {}", j, i),
            )
            .await
            .expect("Failed to insert document");
        }
    }

    // Spawn concurrent queries from different tenant contexts
    let mut handles = vec![];

    for tenant in tenants.iter() {
        let pool_clone = test_pool.clone();
        let tenant_clone = *tenant;

        let handle = tokio::spawn(async move {
            let count = count_documents_as_tenant(&pool_clone, tenant_clone)
                .await
                .expect("Failed to count documents");
            (tenant_clone, count)
        });

        handles.push(handle);
    }

    for handle in handles {
        let (tenant_id, doc_count) = handle.await.expect("Task failed");
        assert_eq!(
            doc_count, 3,
            "Tenant {} should see exactly 3 documents with RLS",
            tenant_id
        );
    }
}

#[tokio::test]
async fn test_rls_performance_overhead() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_id = Uuid::new_v4();
    let num_docs = 100;

    // Insert using admin pool
    for i in 0..num_docs {
        let doc_id = Uuid::new_v4();
        admin_insert_document(
            &admin_pool,
            doc_id,
            tenant_id,
            &format!("Perf Test Doc {}", i),
            &format!("Content for performance testing document {}", i),
        )
        .await
        .expect("Failed to insert document");
    }

    // Test with RLS enforcement (transaction-local GUC)
    let mut conn = test_pool
        .acquire()
        .await
        .expect("Failed to acquire connection");
    begin_tenant_tx(&mut conn, tenant_id)
        .await
        .expect("Failed to begin tenant tx");

    let start = std::time::Instant::now();

    for _ in 0..100 {
        let _docs: Vec<(Uuid, String)> =
            sqlx::query_as("SELECT id, title FROM documents ORDER BY title LIMIT 10")
                .fetch_all(&mut *conn)
                .await
                .expect("Query failed");
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / 100.0;
    commit_tx(&mut conn).await.expect("commit");

    println!(
        "Average query time with RLS enforcement: {:.2}ms ({} queries over {} documents)",
        avg_ms, 100, num_docs
    );

    assert!(
        avg_ms < 50.0,
        "RLS query performance should be < 50ms, got {:.2}ms",
        avg_ms
    );
}

// ---------------------------------------------------------------------------
// SPEC-083 matrix Cluster 01 — named e2e assertions
// ---------------------------------------------------------------------------

/// `e2e_rls_guc_visible_on_following_insert`: BEGIN + is_local=true makes GUC
/// visible to a following INSERT in the same transaction.
#[tokio::test]
async fn e2e_rls_guc_visible_on_following_insert() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant = Uuid::new_v4();
    let doc_id = Uuid::new_v4();
    ensure_tenant(&admin_pool, tenant)
        .await
        .expect("ensure tenant");
    let mut conn = test_pool.acquire().await.expect("acquire");
    begin_tenant_tx(&mut conn, tenant)
        .await
        .expect("begin tenant tx");

    // GUC must be visible on the next statement in this transaction.
    let guc: (String,) = sqlx::query_as("SELECT current_setting('app.current_tenant_id', true)")
        .fetch_one(&mut *conn)
        .await
        .expect("read guc");
    assert_eq!(guc.0, tenant.to_string());

    sqlx::query(
        "INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, 'guc-vis', 'x')",
    )
    .bind(doc_id)
    .bind(tenant)
    .execute(&mut *conn)
    .await
    .expect("insert under local GUC");
    commit_tx(&mut conn).await.expect("commit");

    let count = count_documents_as_tenant(&test_pool, tenant)
        .await
        .expect("count");
    assert_eq!(count, 1);
}

/// `e2e_owner_forced_rls`: FORCE RLS means even the table owner cannot bypass
/// when connecting as a non-bypass role (app_user). Superuser admin still sees all.
#[tokio::test]
async fn e2e_owner_forced_rls() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();
    admin_insert_document(&admin_pool, doc_b, tenant_b, "secret", "x")
        .await
        .expect("insert");
    ensure_tenant(&admin_pool, tenant_a)
        .await
        .expect("ensure tenant A");

    // Without tenant GUC, app_user must see 0 rows (FORCE + fail-closed).
    let mut conn = test_pool.acquire().await.expect("acquire");
    sqlx::query("BEGIN")
        .execute(&mut *conn)
        .await
        .expect("begin");
    // Clear any leftover GUC
    let _ = sqlx::query("SELECT set_config('app.current_tenant_id', '', true)")
        .execute(&mut *conn)
        .await;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    rollback_tx(&mut conn).await.expect("rollback");
    assert_eq!(
        count.0, 0,
        "FORCE RLS: app role without tenant GUC must see 0 documents"
    );

    // With wrong tenant, still invisible
    let title = get_document_title_as_tenant(&test_pool, tenant_a, doc_b)
        .await
        .expect("query");
    assert!(title.is_none());

    // Admin (bypass) still sees the row
    let admin_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE id = $1")
        .bind(doc_b)
        .fetch_one(&admin_pool)
        .await
        .expect("admin");
    assert_eq!(admin_count.0, 1);
}

/// `e2e_null_tenant_row_invisible`: NULL tenant_id rows are not world-readable.
#[tokio::test]
async fn e2e_null_tenant_row_invisible() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let doc_id = Uuid::new_v4();
    // Insert with NULL tenant via admin (may fail if NOT NULL constraint — skip gracefully)
    let insert = sqlx::query(
        "INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, NULL, 'orphan', 'x')",
    )
    .bind(doc_id)
    .execute(&admin_pool)
    .await;

    if insert.is_err() {
        // Schema rejects NULL tenant — invariant holds at DDL level.
        return;
    }

    let any_tenant = Uuid::new_v4();
    let title = get_document_title_as_tenant(&test_pool, any_tenant, doc_id)
        .await
        .expect("query");
    assert!(
        title.is_none(),
        "NULL tenant_id row must not be visible under fail-closed RLS"
    );

    // No GUC → still invisible
    let mut conn = test_pool.acquire().await.expect("acquire");
    sqlx::query("BEGIN")
        .execute(&mut *conn)
        .await
        .expect("begin");
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE id = $1")
        .bind(doc_id)
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    rollback_tx(&mut conn).await.expect("rollback");
    assert_eq!(count.0, 0);
}

/// `e2e_document_originals_cross_workspace_denied`: binary originals are workspace-scoped.
#[tokio::test]
async fn e2e_document_originals_cross_workspace_denied() {
    let Some((admin_pool, test_pool)) = require_rls_rig().await else {
        return;
    };

    let exists: (bool,) =
        sqlx::query_as("SELECT to_regclass('public.document_originals') IS NOT NULL")
            .fetch_one(&admin_pool)
            .await
            .unwrap_or((false,));
    assert!(exists.0, "required binary-originals table is missing");

    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();
    let tenant = Uuid::new_v4();
    let doc_id = Uuid::new_v4();

    ensure_workspace(&admin_pool, ws_a, tenant)
        .await
        .expect("ensure ws a");
    ensure_workspace(&admin_pool, ws_b, tenant)
        .await
        .expect("ensure ws b");

    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content) VALUES ($1, $2, $3, 'orig', 'x')",
    )
    .bind(doc_id)
    .bind(tenant)
    .bind(ws_b)
    .execute(&admin_pool)
    .await
    .expect("insert document for originals");

    sqlx::query(
        r#"
        INSERT INTO document_originals
            (document_id, workspace_id, filename, content_type, file_size_bytes, original_data)
        VALUES ($1, $2, 'secret.bin', 'application/octet-stream', 4, '\x25504446'::bytea)
        ON CONFLICT (document_id) DO UPDATE SET workspace_id = EXCLUDED.workspace_id
        "#,
    )
    .bind(doc_id)
    .bind(ws_b)
    .execute(&admin_pool)
    .await
    .expect("seed binary originals");

    let mut conn = test_pool.acquire().await.expect("acquire");
    begin_tenant_tx(&mut conn, tenant)
        .await
        .expect("tenant scope");
    sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
        .bind(ws_a.to_string())
        .execute(&mut *conn)
        .await
        .expect("set ws");
    let seen: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM document_originals WHERE document_id = $1")
            .bind(doc_id)
            .fetch_one(&mut *conn)
            .await
            .expect("read originals under RLS");
    assert_eq!(
        seen.0, 0,
        "workspace A must not see originals belonging to workspace B"
    );

    sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
        .bind(ws_b.to_string())
        .execute(&mut *conn)
        .await
        .expect("own workspace");
    let own: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_originals WHERE document_id=$1")
            .bind(doc_id)
            .fetch_one(&mut *conn)
            .await
            .expect("own originals read");
    assert_eq!(own, 1, "own binary original must remain readable");
    rollback_tx(&mut conn).await.expect("rollback");

    let _ = sqlx::query("DELETE FROM document_originals WHERE document_id = $1")
        .bind(doc_id)
        .execute(&admin_pool)
        .await;
    let _ = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(&admin_pool)
        .await;
}
