//! A late keyset page stops after the cursor.
#![cfg(feature = "postgres")]

#[path = "support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use edgequake_storage::adapters::postgres::document_shell::shell_staging_keys_page;
use edgequake_storage::PgIngestionCommitter;
use edgequake_storage_contracts::{
    AccessScope, DocumentPageRequest, DocumentReader, TenantId, WorkspaceId,
};
use uuid::Uuid;

async fn scope(pool: &sqlx::PgPool) -> (Uuid, Uuid) {
    let (tenant, workspace) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO tenants(tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant)
        .bind("keyset")
        .bind(tenant.to_string())
        .execute(pool)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO workspaces(workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind("keyset")
    .bind(workspace.to_string())
    .execute(pool)
    .await
    .expect("workspace");
    (tenant, workspace)
}

#[tokio::test]
async fn staging_shell_page_after_a_cursor_skips_that_id() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool).await;
    let mut ids = [Uuid::new_v4(), Uuid::new_v4()];
    ids.sort();
    let [left, right] = ids;
    for id in [left, right] {
        sqlx::query(
            "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, metadata)
             VALUES ($1, $2, $3, 'shell', '', 'completed', '{\"_shell\":\"staging\"}'::jsonb)",
        )
        .bind(id)
        .bind(tenant)
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("shell");
    }

    let page = shell_staging_keys_page(&pool, Some(left), 5000)
        .await
        .expect("page");
    assert!(page.iter().any(|key| key.contains(&right.to_string())));
    assert!(page.iter().all(|key| !key.contains(&left.to_string())));

    sqlx::query("DELETE FROM documents WHERE id = ANY($1)")
        .bind(vec![left, right])
        .execute(&pool)
        .await
        .expect("cleanup");
}

#[tokio::test]
async fn revision_page_after_a_cursor_skips_that_id() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool).await;
    let mut ids = [Uuid::new_v4(), Uuid::new_v4()];
    ids.sort();
    let [left, right] = ids;
    let digest = vec![0xab_u8; 32];
    for id in [left, right] {
        sqlx::query(
            "INSERT INTO object_revisions (
                tenant_id, workspace_id, kind, logical_id, revision, state, physical_id, digest, payload
             ) VALUES ($1, $2, 'document', $3, 1, 'active', $4, $5, ''::bytea)",
        )
        .bind(tenant)
        .bind(workspace)
        .bind(id)
        .bind(Uuid::new_v4())
        .bind(&digest)
        .execute(&pool)
        .await
        .expect("revision");
    }

    let reader = PgIngestionCommitter::new(pool.clone());
    let scope = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace));
    let page = reader
        .list(
            &scope,
            &DocumentPageRequest {
                cursor: Some(left.to_string()),
                limit: 10,
            },
        )
        .await
        .expect("page");
    let ids: Vec<Uuid> = page
        .items
        .iter()
        .map(|item| item.document_id.into_uuid())
        .collect();
    assert_eq!(ids, vec![right]);

    sqlx::query("DELETE FROM object_revisions WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("cleanup");
}
