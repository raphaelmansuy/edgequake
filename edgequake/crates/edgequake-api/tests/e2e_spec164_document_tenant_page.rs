//! A tenant's document page includes rows with no tenant and skips another
//! tenant's newer rows in the same workspace.
#![cfg(feature = "postgres")]

#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use edgequake_api::document_read_model::{
    count_relational_document_statuses, delete_relational_documents_for_workspace,
    filter_relational_document_candidates, list_relational_document_summaries_limited,
    RelationalCreatedBounds,
};
use edgequake_api::middleware::TenantContext;
use uuid::Uuid;

async fn scope(pool: &sqlx::PgPool, name: &str) -> (Uuid, Uuid) {
    let (tenant, workspace) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO tenants(tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant)
        .bind(name)
        .bind(tenant.to_string())
        .execute(pool)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO workspaces(workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(name)
    .bind(workspace.to_string())
    .execute(pool)
    .await
    .expect("workspace");
    (tenant, workspace)
}

async fn insert_doc(
    pool: &sqlx::PgPool,
    tenant: Option<Uuid>,
    workspace: Uuid,
    title: &str,
    age: &str,
) {
    sqlx::query(&format!(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, '', 'completed', now() - interval '{age}', now())"
    ))
    .bind(Uuid::new_v4())
    .bind(tenant)
    .bind(workspace)
    .bind(title)
    .execute(pool)
    .await
    .expect("document");
}

fn ctx(tenant: Option<Uuid>, workspace: Uuid) -> TenantContext {
    TenantContext {
        tenant_id: tenant.map(|id| id.to_string()),
        workspace_id: Some(workspace.to_string()),
        user_id: None,
    }
}

#[tokio::test]
async fn tenant_document_page_keeps_unset_tenant_and_skips_the_other_tenant() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant_a, workspace) = scope(&pool, "doc page a").await;
    let (tenant_b, _other_workspace) = scope(&pool, "doc page b").await;
    insert_doc(&pool, None, workspace, "unset", "1 minute").await;
    insert_doc(&pool, Some(tenant_b), workspace, "other", "2 minutes").await;
    insert_doc(&pool, Some(tenant_a), workspace, "own", "3 minutes").await;

    let scoped = ctx(Some(tenant_a), workspace);
    let page = list_relational_document_summaries_limited(
        Some(&pool),
        &scoped,
        10,
        None,
        RelationalCreatedBounds::default(),
        &[],
    )
    .await
    .expect("scoped page");
    let titles: Vec<_> = page
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(titles, vec!["unset".to_string(), "own".to_string()]);
    assert!(!page.truncated);

    let counts = count_relational_document_statuses(Some(&pool), &scoped)
        .await
        .expect("scoped counts");
    assert_eq!(counts.completed, 2);

    let open = ctx(None, workspace);
    let all = list_relational_document_summaries_limited(
        Some(&pool),
        &open,
        10,
        None,
        RelationalCreatedBounds::default(),
        &[],
    )
    .await
    .expect("workspace page");
    let all_titles: Vec<_> = all
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(
        all_titles,
        vec!["unset".to_string(), "other".to_string(), "own".to_string()]
    );

    let removed = delete_relational_documents_for_workspace(Some(&pool), &scoped)
        .await
        .expect("scoped delete");
    assert_eq!(removed, 2, "unset and own rows leave with tenant A");
    let left: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM documents WHERE workspace_id = $1 AND title = 'other'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("other tenant row");
    assert_eq!(left, 1);

    sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1 OR tenant_id = $2")
        .bind(workspace)
        .bind(tenant_b)
        .execute(&pool)
        .await
        .expect("cleanup workspaces");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1 OR tenant_id = $2")
        .bind(tenant_a)
        .bind(tenant_b)
        .execute(&pool)
        .await
        .expect("cleanup tenants");
}

#[tokio::test]
async fn document_page_date_to_returns_the_older_row() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool, "doc date page").await;
    let older = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let newer = chrono::DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    insert_doc_at(&pool, Some(tenant), workspace, "older", older).await;
    insert_doc_at(&pool, Some(tenant), workspace, "newer", newer).await;

    let scoped = ctx(Some(tenant), workspace);
    let page = list_relational_document_summaries_limited(
        Some(&pool),
        &scoped,
        10,
        None,
        RelationalCreatedBounds {
            from: None,
            to: Some(older),
        },
        &[],
    )
    .await
    .expect("dated page");
    let titles: Vec<_> = page
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(titles, vec!["older".to_string()]);
    assert!(!page.truncated);

    let open = list_relational_document_summaries_limited(
        Some(&pool),
        &scoped,
        10,
        None,
        RelationalCreatedBounds::default(),
        &[],
    )
    .await
    .expect("open page");
    let open_titles: Vec<_> = open
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(open_titles, vec!["newer".to_string(), "older".to_string()]);

    sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("cleanup tenant");
}

#[tokio::test]
async fn document_page_unknown_returns_the_older_deleting_row() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool, "doc unknown page").await;
    let older = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let newer = chrono::DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    insert_doc_status(
        &pool,
        Some(tenant),
        workspace,
        "older-unknown",
        "deleting",
        older,
    )
    .await;
    insert_doc_status(
        &pool,
        Some(tenant),
        workspace,
        "newer-done",
        "completed",
        newer,
    )
    .await;

    let scoped = ctx(Some(tenant), workspace);
    let page = list_relational_document_summaries_limited(
        Some(&pool),
        &scoped,
        10,
        Some("unknown"),
        RelationalCreatedBounds::default(),
        &[],
    )
    .await
    .expect("unknown page");
    let titles: Vec<_> = page
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(titles, vec!["older-unknown".to_string()]);
    assert_eq!(
        page.documents[0].status.as_deref(),
        Some("deleting"),
        "deleting stays unknown to the list filter"
    );
    assert!(!page.truncated);

    let open = list_relational_document_summaries_limited(
        Some(&pool),
        &scoped,
        10,
        Some("all"),
        RelationalCreatedBounds::default(),
        &[],
    )
    .await
    .expect("all page");
    let open_titles: Vec<_> = open
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(
        open_titles,
        vec!["newer-done".to_string(), "older-unknown".to_string()]
    );

    sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("cleanup tenant");
}

#[tokio::test]
async fn document_page_title_pattern_returns_the_matching_row() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool, "doc pattern page").await;
    let older = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let newer = chrono::DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    insert_doc_at(&pool, Some(tenant), workspace, "uniquezephyr", older).await;
    insert_doc_at(&pool, Some(tenant), workspace, "plain", newer).await;

    let scoped = ctx(Some(tenant), workspace);
    let patterns = vec!["uniquezephyr".to_string()];
    let page = list_relational_document_summaries_limited(
        Some(&pool),
        &scoped,
        10,
        None,
        RelationalCreatedBounds::default(),
        &patterns,
    )
    .await
    .expect("pattern page");
    let titles: Vec<_> = page
        .documents
        .iter()
        .filter_map(|doc| doc.title.clone())
        .collect();
    assert_eq!(titles, vec!["uniquezephyr".to_string()]);

    sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("cleanup tenant");
}

#[tokio::test]
async fn query_filter_date_returns_only_the_older_document() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool, "query filter date").await;
    let older = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let newer = chrono::DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    insert_doc_at(&pool, Some(tenant), workspace, "uniquezephyr", older).await;
    insert_doc_at(&pool, Some(tenant), workspace, "plain", newer).await;

    let dated = filter_relational_document_candidates(
        &pool,
        workspace,
        RelationalCreatedBounds {
            from: None,
            to: Some(older),
        },
        &[],
        &[],
    )
    .await
    .expect("dated ids");
    let titles: Vec<_> = dated.into_iter().map(|hit| hit.title).collect();
    assert_eq!(titles, vec!["uniquezephyr".to_string()]);

    let patterned = filter_relational_document_candidates(
        &pool,
        workspace,
        RelationalCreatedBounds::default(),
        &["uniquezephyr".to_string()],
        &[],
    )
    .await
    .expect("pattern ids");
    let titles: Vec<_> = patterned.into_iter().map(|hit| hit.title).collect();
    assert_eq!(titles, vec!["uniquezephyr".to_string()]);

    sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup documents");
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("cleanup tenant");
}

async fn insert_doc_status(
    pool: &sqlx::PgPool,
    tenant: Option<Uuid>,
    workspace: Uuid,
    title: &str,
    status: &str,
    created_at: chrono::DateTime<chrono::Utc>,
) {
    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, '', $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(tenant)
    .bind(workspace)
    .bind(title)
    .bind(status)
    .bind(created_at)
    .bind(created_at)
    .execute(pool)
    .await
    .expect("status document");
}

async fn insert_doc_at(
    pool: &sqlx::PgPool,
    tenant: Option<Uuid>,
    workspace: Uuid,
    title: &str,
    created_at: chrono::DateTime<chrono::Utc>,
) {
    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, '', 'indexed', $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(tenant)
    .bind(workspace)
    .bind(title)
    .bind(created_at)
    .bind(created_at)
    .execute(pool)
    .await
    .expect("dated document");
}
