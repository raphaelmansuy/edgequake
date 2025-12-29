//! PostgreSQL Row Level Security (RLS) E2E Tests
//!
//! These tests verify tenant isolation at the database level using PostgreSQL RLS.
//! Run with: cargo test --package edgequake-api --test e2e_postgres_rls -- --ignored
//!
//! Key Insights:
//! 1. Superusers ALWAYS bypass RLS - we use app_user (non-superuser) for testing
//! 2. set_config is session-scoped - we must use the same connection for set and query
//! 3. Connection pools give different connections - use acquire() for dedicated connection

use sqlx::{postgres::PgPoolOptions, Acquire, Pool, Postgres};
use uuid::Uuid;

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

/// Clean test data using admin pool
async fn clean_test_data(admin_pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query("TRUNCATE TABLE documents CASCADE")
        .execute(admin_pool)
        .await?;
    Ok(())
}

/// Query with tenant context on a dedicated connection
/// This ensures set_config and query use the same session
async fn query_with_tenant_context<T>(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    query: &str,
) -> Result<T, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    // Get a dedicated connection from the pool
    let mut conn = pool.acquire().await?;

    // Set tenant context
    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await?;

    // Execute query on same connection
    let result = sqlx::query_as::<_, T>(query).fetch_one(&mut *conn).await?;

    Ok(result)
}

/// Count documents with tenant context
async fn count_documents_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let mut conn = pool.acquire().await?;

    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
        .fetch_one(&mut *conn)
        .await?;

    Ok(count.0)
}

/// Execute UPDATE with tenant context, returns rows affected
async fn update_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    query: &str,
    bind_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let mut conn = pool.acquire().await?;

    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await?;

    let result = sqlx::query(query).bind(bind_id).execute(&mut *conn).await?;

    Ok(result.rows_affected())
}

/// Execute DELETE with tenant context, returns rows affected
async fn delete_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    doc_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let mut conn = pool.acquire().await?;

    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await?;

    let result = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(&mut *conn)
        .await?;

    Ok(result.rows_affected())
}

/// Query document by ID with tenant context
async fn get_document_title_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    doc_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let mut conn = pool.acquire().await?;

    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await?;

    let result: Option<(String,)> = sqlx::query_as("SELECT title FROM documents WHERE id = $1")
        .bind(doc_id)
        .fetch_optional(&mut *conn)
        .await?;

    Ok(result.map(|r| r.0))
}

/// Insert document with tenant context
async fn insert_as_tenant(
    pool: &Pool<Postgres>,
    tenant_id: Uuid,
    doc_id: Uuid,
    insert_tenant_id: Uuid,
    title: &str,
) -> Result<(), sqlx::Error> {
    let mut conn = pool.acquire().await?;

    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await?;

    sqlx::query(
        "INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, $3, 'Content')",
    )
    .bind(doc_id)
    .bind(insert_tenant_id)
    .bind(title)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_postgres_rls_basic_isolation() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_a = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    // Insert using admin pool (bypasses RLS)
    sqlx::query("INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, 'Doc A', 'Content A')")
        .bind(doc_a)
        .bind(tenant_a)
        .execute(&admin_pool)
        .await
        .expect("Failed to insert doc A");

    sqlx::query("INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, 'Doc B', 'Content B')")
        .bind(doc_b)
        .bind(tenant_b)
        .execute(&admin_pool)
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

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}

#[tokio::test]
#[ignore]
async fn test_postgres_rls_cross_tenant_query_blocked() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    // Insert document for tenant B
    sqlx::query("INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, 'Secret B', 'Confidential')")
        .bind(doc_b)
        .bind(tenant_b)
        .execute(&admin_pool)
        .await
        .expect("Failed to insert doc B");

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

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}

#[tokio::test]
#[ignore]
async fn test_postgres_update_isolation() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    // Insert document for tenant B
    sqlx::query("INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, 'Original B', 'Content')")
        .bind(doc_b)
        .bind(tenant_b)
        .execute(&admin_pool)
        .await
        .expect("Failed to insert doc B");

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

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}

#[tokio::test]
#[ignore]
async fn test_postgres_delete_isolation() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_b = Uuid::new_v4();

    // Insert document for tenant B
    sqlx::query("INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, 'Keep B', 'Content')")
        .bind(doc_b)
        .bind(tenant_b)
        .execute(&admin_pool)
        .await
        .expect("Failed to insert doc B");

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

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}

#[tokio::test]
#[ignore]
async fn test_rls_insert_isolation() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let doc_id = Uuid::new_v4();

    // Test: Tenant A tries to insert a document with Tenant B's ID
    let result = insert_as_tenant(&test_pool, tenant_a, doc_id, tenant_b, "Sneaky").await;

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
    let result_valid =
        insert_as_tenant(&test_pool, tenant_a, doc_id_valid, tenant_a, "Valid").await;

    assert!(
        result_valid.is_ok(),
        "Tenant A should be able to insert with their own tenant_id"
    );

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}

#[tokio::test]
#[ignore]
async fn test_tenant_isolation_with_concurrent_access() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    // Create 5 tenants with 3 documents each
    let tenants: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    // Insert using admin pool
    for (i, tenant) in tenants.iter().enumerate() {
        for j in 0..3 {
            let doc_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, $3, $4)",
            )
            .bind(doc_id)
            .bind(tenant)
            .bind(format!("Doc {} for Tenant {}", j, i))
            .bind(format!("Content {} for Tenant {}", j, i))
            .execute(&admin_pool)
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

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}

#[tokio::test]
#[ignore]
async fn test_rls_performance_overhead() {
    let admin_pool = create_admin_pool()
        .await
        .expect("Failed to create admin pool");
    let test_pool = create_test_pool()
        .await
        .expect("Failed to create test pool");

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");

    let tenant_id = Uuid::new_v4();
    let num_docs = 100;

    // Insert using admin pool
    for i in 0..num_docs {
        let doc_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO documents (id, tenant_id, title, content) VALUES ($1, $2, $3, $4)",
        )
        .bind(doc_id)
        .bind(tenant_id)
        .bind(format!("Perf Test Doc {}", i))
        .bind(format!("Content for performance testing document {}", i))
        .execute(&admin_pool)
        .await
        .expect("Failed to insert document");
    }

    // Test with RLS enforcement
    let mut conn = test_pool
        .acquire()
        .await
        .expect("Failed to acquire connection");

    sqlx::query("SELECT set_config('app.current_tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await
        .expect("Failed to set context");

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

    println!(
        "Average query time with RLS enforcement: {:.2}ms ({} queries over {} documents)",
        avg_ms, 100, num_docs
    );

    assert!(
        avg_ms < 50.0,
        "RLS query performance should be < 50ms, got {:.2}ms",
        avg_ms
    );

    clean_test_data(&admin_pool)
        .await
        .expect("Failed to clean data");
}
