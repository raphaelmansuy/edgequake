//! Dedup staging reserve must satisfy `documents.tenant_access_guard`.
//!
//! The guard is a restrictive WITH CHECK: the inserted `tenant_id` must equal
//! `current_tenant_id()`, and `workspace_id` must equal `current_workspace_id()`
//! when that session setting is present. A pool checkout that never calls
//! `enforce_tenant_context` leaves both settings null, so the parent insert
//! fails. A legitimate owner passes. A tenant that does not own the workspace
//! is rejected, and a row whose tenant differs from the installed session is
//! still rejected by the policy.
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::adapters::postgres::document_shell::dual_write_shell_upserts;
use edgequake_storage::adapters::postgres::ingestion_dedup::{
    upsert_reservation, DEDUP_VERSION_STAGING,
};
use edgequake_storage::adapters::postgres::rls::with_rls_transaction;
use edgequake_storage::StorageError;
use serde_json::json;
use sqlx::postgres::PgConnectOptions;
use sqlx::PgPool;
use uuid::Uuid;

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

const WRITER_ROLE: &str = "eq_dedup_rls_writer";
const WRITER_PASSWORD: &str = "dedup_rls_writer_pw";

#[tokio::test]
async fn dedup_reserve_honors_tenant_access_guard() {
    let Some(config) = require_or_skip_postgres("dedup_rls") else {
        return;
    };
    let admin = contract_pg_pool(&config).await;
    let guard: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_policies \
         WHERE schemaname = 'public' AND tablename = 'documents' \
           AND policyname = 'tenant_access_guard'",
    )
    .fetch_one(&admin)
    .await
    .expect("policy lookup");
    assert_eq!(guard, 1, "documents.tenant_access_guard must be installed");

    provision_writer(&admin).await;
    let writer = writer_pool(&config).await;
    let bypass: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&writer)
    .await
    .expect("writer role flags");
    assert!(!bypass, "fixture role must be subject to RLS");

    let tenant = Uuid::new_v4();
    let other = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let document = Uuid::new_v4();
    seed_scope(&admin, tenant, workspace, "owner").await;
    seed_tenant(&admin, other, "foreign").await;

    let bare = Uuid::new_v4();
    let bare_insert = sqlx::query(
        "INSERT INTO public.documents (id, tenant_id, workspace_id, content, status) \
         VALUES ($1, $2, $3, '', 'processing')",
    )
    .bind(bare)
    .bind(tenant)
    .bind(workspace)
    .execute(&writer)
    .await;
    let bare_err = bare_insert.expect_err("unset session tenant must fail the guard");
    let bare_msg = bare_err.to_string();
    assert!(
        bare_msg.contains("tenant_access_guard"),
        "bare insert should name tenant_access_guard, got: {bare_msg}"
    );

    let mismatch = upsert_reservation(
        &writer,
        &workspace.to_string(),
        "hash-mismatch",
        DEDUP_VERSION_STAGING,
        &document.to_string(),
        Some(&other.to_string()),
    )
    .await
    .expect_err("foreign tenant must not reserve");
    let mismatch_msg = mismatch.to_string();
    assert!(
        mismatch_msg.contains("does not own") || mismatch_msg.contains("tenant_access_guard"),
        "mismatch rejection should be ownership or the guard, got: {mismatch_msg}"
    );
    let leaked: i64 = sqlx::query_scalar("SELECT count(*) FROM public.documents WHERE id = $1")
        .bind(document)
        .fetch_one(&admin)
        .await
        .expect("document count");
    assert_eq!(
        leaked, 0,
        "rejected reserve must not insert a document parent"
    );

    upsert_reservation(
        &writer,
        &workspace.to_string(),
        "hash-ok",
        DEDUP_VERSION_STAGING,
        &document.to_string(),
        Some(&tenant.to_string()),
    )
    .await
    .expect("owner reserve");

    let stored: (Uuid, Uuid) =
        sqlx::query_as("SELECT tenant_id, workspace_id FROM public.documents WHERE id = $1")
            .bind(document)
            .fetch_one(&admin)
            .await
            .expect("parent row");
    assert_eq!(stored, (tenant, workspace));

    let reserved: Uuid = sqlx::query_scalar(
        "SELECT document_id FROM public.ingestion_dedup \
         WHERE workspace_id = $1 AND content_hash = $2 AND pipeline_version = $3",
    )
    .bind(workspace)
    .bind("hash-ok")
    .bind(DEDUP_VERSION_STAGING)
    .fetch_one(&admin)
    .await
    .expect("staging reservation");
    assert_eq!(reserved, document);

    let forged = Uuid::new_v4();
    let rejected = with_rls_transaction(&writer, tenant, Some(workspace), None, move |conn| {
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO public.documents (id, tenant_id, workspace_id, content, status) \
                 VALUES ($1, $2, $3, '', 'processing')",
            )
            .bind(forged)
            .bind(other)
            .bind(workspace)
            .execute(&mut *conn)
            .await
            .map_err(StorageError::from)?;
            Ok(())
        })
    })
    .await
    .expect_err("row tenant must match the session tenant");
    let rejected_msg = rejected.to_string();
    assert!(
        rejected_msg.contains("tenant_access_guard"),
        "forged tenant must still hit tenant_access_guard, got: {rejected_msg}"
    );

    sqlx::query("DELETE FROM public.ingestion_dedup WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&admin)
        .await
        .expect("cleanup dedup");
    sqlx::query("DELETE FROM public.documents WHERE id = $1 OR id = $2")
        .bind(document)
        .bind(bare)
        .execute(&admin)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM public.workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&admin)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM public.tenants WHERE tenant_id = $1 OR tenant_id = $2")
        .bind(tenant)
        .bind(other)
        .execute(&admin)
        .await
        .expect("cleanup tenants");
}

#[tokio::test]
async fn staging_shell_upsert_satisfies_tenant_access_guard() {
    let Some(config) = require_or_skip_postgres("shell_rls") else {
        return;
    };
    let admin = contract_pg_pool(&config).await;
    provision_writer(&admin).await;
    let writer = writer_pool_limited(&config, 1).await;
    let bypass: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&writer)
    .await
    .expect("writer role flags");
    assert!(!bypass, "fixture role must be subject to RLS");

    let tenant = Uuid::new_v4();
    let other = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let document = Uuid::new_v4();
    let foreign_doc = Uuid::new_v4();
    seed_scope(&admin, tenant, workspace, "shell").await;
    seed_tenant(&admin, other, "shell-foreign").await;

    sqlx::query(
        "SELECT set_config('app.current_tenant_id', $1, false), \
                set_config('app.current_workspace_id', $2, false)",
    )
    .bind(tenant.to_string())
    .bind(workspace.to_string())
    .execute(&writer)
    .await
    .expect("install session tenant");

    dual_write_shell_upserts(
        &writer,
        &[(
            format!("staging:{document}-metadata"),
            json!({
                "title": "eq-raw-probe.md",
                "status": "pending",
                "tenant_id": tenant.to_string(),
                "workspace_id": workspace.to_string(),
            }),
        )],
        true,
    )
    .await
    .expect("owner staging shell");

    dual_write_shell_upserts(
        &writer,
        &[(
            format!("staging:{document}-content"),
            json!({ "content": "markdown body" }),
        )],
        true,
    )
    .await
    .expect("owner staging content");

    let stored: (String, String, String, Uuid, Uuid) = sqlx::query_as(
        "SELECT title, content, metadata->>'_shell', tenant_id, workspace_id \
         FROM public.documents WHERE id = $1",
    )
    .bind(document)
    .fetch_one(&admin)
    .await
    .expect("shell row");
    assert_eq!(
        stored,
        (
            "eq-raw-probe.md".to_string(),
            "markdown body".to_string(),
            "staging".to_string(),
            tenant,
            workspace,
        )
    );

    let rejected = dual_write_shell_upserts(
        &writer,
        &[(
            format!("staging:{foreign_doc}-metadata"),
            json!({
                "title": "foreign",
                "tenant_id": other.to_string(),
                "workspace_id": workspace.to_string(),
            }),
        )],
        true,
    )
    .await
    .expect_err("foreign tenant must not write a document shell");
    let rejected_msg = rejected.to_string();
    assert!(
        rejected_msg.contains("tenant_access_guard"),
        "foreign shell must still hit tenant_access_guard, got: {rejected_msg}"
    );
    let leaked: i64 = sqlx::query_scalar("SELECT count(*) FROM public.documents WHERE id = $1")
        .bind(foreign_doc)
        .fetch_one(&admin)
        .await
        .expect("foreign document count");
    assert_eq!(leaked, 0, "rejected shell must not insert a document");

    sqlx::query("DELETE FROM public.documents WHERE id = $1 OR id = $2")
        .bind(document)
        .bind(foreign_doc)
        .execute(&admin)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM public.workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&admin)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM public.tenants WHERE tenant_id = $1 OR tenant_id = $2")
        .bind(tenant)
        .bind(other)
        .execute(&admin)
        .await
        .expect("cleanup tenants");
}

async fn provision_writer(admin: &PgPool) {
    sqlx::query(&format!(
        "DO $$ BEGIN \
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{WRITER_ROLE}') THEN \
             CREATE ROLE {WRITER_ROLE} LOGIN PASSWORD '{WRITER_PASSWORD}' \
               NOSUPERUSER NOBYPASSRLS NOINHERIT; \
           END IF; \
         END $$"
    ))
    .execute(admin)
    .await
    .expect("create writer role");
    sqlx::query(&format!(
        "ALTER ROLE {WRITER_ROLE} WITH LOGIN PASSWORD '{WRITER_PASSWORD}' \
         NOSUPERUSER NOBYPASSRLS NOINHERIT"
    ))
    .execute(admin)
    .await
    .expect("pin writer role");
    // Production sessions that already are `edgequake_tenant_access` (the
    // permissive allow passes) fail `tenant_access_guard` by name when the
    // tenant GUC is unset. Pin that default so the fixture matches the error.
    sqlx::query(&format!(
        "ALTER ROLE {WRITER_ROLE} SET role TO 'edgequake_tenant_access'"
    ))
    .execute(admin)
    .await
    .expect("default writer role");
    let db: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(admin)
        .await
        .expect("database name");
    let grants = [
        format!("GRANT CONNECT ON DATABASE \"{db}\" TO {WRITER_ROLE}"),
        format!("GRANT USAGE ON SCHEMA public TO {WRITER_ROLE}"),
        format!("GRANT SELECT ON public.tenants, public.workspaces TO {WRITER_ROLE}"),
        format!(
            "GRANT SELECT, INSERT ON public.documents, public.ingestion_dedup TO {WRITER_ROLE}"
        ),
        format!("GRANT edgequake_tenant_access TO {WRITER_ROLE}"),
    ];
    for stmt in grants {
        sqlx::query(&stmt)
            .execute(admin)
            .await
            .unwrap_or_else(|error| panic!("grant failed ({stmt}): {error}"));
    }
}

async fn writer_pool(config: &edgequake_storage::PostgresConfig) -> PgPool {
    writer_pool_limited(config, 2).await
}

async fn writer_pool_limited(config: &edgequake_storage::PostgresConfig, max: u32) -> PgPool {
    let options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .database(&config.database)
        .username(WRITER_ROLE)
        .password(WRITER_PASSWORD);
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(max)
        .connect_with(options)
        .await
        .expect("writer pool")
}

async fn seed_tenant(pool: &PgPool, tenant_id: Uuid, label: &str) {
    let slug = format!("dedup-rls-{label}-{}", tenant_id.as_simple());
    sqlx::query(
        "INSERT INTO public.tenants (tenant_id, name, slug) VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(format!("dedup rls {label}"))
    .bind(slug)
    .execute(pool)
    .await
    .expect("seed tenant");
}

async fn seed_scope(pool: &PgPool, tenant_id: Uuid, workspace_id: Uuid, label: &str) {
    seed_tenant(pool, tenant_id, label).await;
    let slug = format!("dedup-rls-ws-{label}-{}", workspace_id.as_simple());
    sqlx::query(
        "INSERT INTO public.workspaces (workspace_id, tenant_id, name, slug) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id) DO NOTHING",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("dedup rls ws {label}"))
    .bind(slug)
    .execute(pool)
    .await
    .expect("seed workspace");
}
