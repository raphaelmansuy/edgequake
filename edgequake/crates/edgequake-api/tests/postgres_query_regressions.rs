//! Execution regressions discovered by the source-derived SQL preparation gate.
#![cfg(feature = "postgres")]

#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

// Retained legacy source is checked by the catalog even when it is outside
// the current application module tree. Test it without enabling it at runtime.
#[path = "../src/services/fenced_write.rs"]
mod fenced_write;
mod services {
    pub use edgequake_api::services::OptionalPgPool;
}

use edgequake_api::PostgresEntitySink;
use edgequake_pipeline::{RelationalEntitySink, RelationshipSinkRow};
use fenced_write::{
    assert_fence, begin_document_run, bind_document_run_track, bump_fence_epoch, FenceError,
};
use std::sync::Arc;
use uuid::Uuid;

async fn scope(pool: &sqlx::PgPool) -> (Uuid, Uuid) {
    let (tenant, workspace) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO tenants(tenant_id,name,slug) VALUES($1,'query regression',$2)")
        .bind(tenant)
        .bind(tenant.to_string())
        .execute(pool)
        .await
        .expect("tenant");
    sqlx::query("INSERT INTO workspaces(workspace_id,tenant_id,name,slug) VALUES($1,$2,'query regression',$3)")
        .bind(workspace).bind(tenant).bind(workspace.to_string()).execute(pool).await.expect("workspace");
    (tenant, workspace)
}

#[tokio::test]
async fn entity_source_update_and_delete_bind_uuid_and_preserve_other_workspaces() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool).await;
    let (other_tenant, other_workspace) = scope(&pool).await;
    let name = format!("O'REILLY_{}", Uuid::new_v4().as_simple());
    let sources = vec!["chunk-a".into(), "chunk-b".into()];
    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));
    for (tenant, ws) in [(tenant, workspace), (other_tenant, other_workspace)] {
        sink.upsert_entity(
            &name,
            "PERSON",
            "query regression",
            Some(&tenant.to_string()),
            Some(&ws.to_string()),
            &sources,
        )
        .await
        .expect("upsert entity");
    }
    let remaining = vec!["chunk-b".into()];
    sink.remove_entity_sources(
        &name,
        Some(&workspace.to_string()),
        &["chunk-a".into()],
        &remaining,
    )
    .await
    .expect("update sources");
    let got: Vec<String> = sqlx::query_scalar(
        "SELECT source_chunk_ids FROM entities WHERE workspace_id=$1 AND name=$2",
    )
    .bind(workspace)
    .bind(&name)
    .fetch_one(&pool)
    .await
    .expect("read sources");
    assert_eq!(got, remaining);
    sink.remove_entity_sources(&name, Some(&workspace.to_string()), &remaining, &[])
        .await
        .expect("delete exhausted entity");
    let scopes: Vec<Uuid> = sqlx::query_scalar("SELECT workspace_id FROM entities WHERE name=$1")
        .bind(&name)
        .fetch_all(&pool)
        .await
        .expect("remaining entity");
    assert_eq!(scopes, vec![other_workspace]);
    sqlx::query("DELETE FROM entities WHERE name=$1")
        .bind(name)
        .execute(&pool)
        .await
        .expect("cleanup entity");
}

#[tokio::test]
async fn fence_track_rebind_updates_metadata_and_rejects_superseded_epoch() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let (tenant, workspace) = scope(&pool).await;
    let document = Uuid::new_v4();
    sqlx::query("INSERT INTO documents(id,tenant_id,workspace_id,title,content) VALUES($1,$2,$3,'query regression','')")
        .bind(document).bind(tenant).bind(workspace).execute(&pool).await.expect("document");
    let id = document.to_string();
    let epoch = begin_document_run(&id, "provisional", "queued", 0, "queued", 0.0, Some(&pool))
        .await
        .expect("start run");
    bind_document_run_track(&id, epoch, "provisional", "durable'quoted", Some(&pool))
        .await
        .expect("bind durable task");
    let actual: (String, String, i64) = sqlx::query_as(
        "SELECT track_id,metadata->>'track_id',fence_epoch FROM documents WHERE id=$1",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .expect("track metadata");
    assert_eq!(
        actual,
        ("durable'quoted".into(), "durable'quoted".into(), epoch.0)
    );
    let next = bump_fence_epoch(&id, Some(&pool)).await.expect("supersede");
    assert_fence(next, &id, Some(&pool))
        .await
        .expect("current fence");
    let stale = bind_document_run_track(&id, epoch, "durable'quoted", "stale", Some(&pool))
        .await
        .expect_err("reject stale epoch");
    assert!(
        matches!(stale, FenceError::Stale { expected, actual } if expected == epoch.0 && actual == next.0)
    );
    let track: String = sqlx::query_scalar("SELECT track_id FROM documents WHERE id=$1")
        .bind(document)
        .fetch_one(&pool)
        .await
        .expect("preserved task");
    assert_eq!(track, "durable'quoted");
    sqlx::query("DELETE FROM documents WHERE id=$1")
        .bind(document)
        .execute(&pool)
        .await
        .expect("cleanup document");
}

#[tokio::test]
async fn relationship_batches_reject_mixed_scopes_and_invalid_scope_ids() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let first = scope(&pool).await;
    let second = scope(&pool).await;
    let names = [
        format!("source_{}", Uuid::new_v4()),
        format!("target_{}", Uuid::new_v4()),
    ];
    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));
    for (tenant, workspace) in [first, second] {
        for name in &names {
            sink.upsert_entity(
                name,
                "PERSON",
                "scoped endpoint",
                Some(&tenant.to_string()),
                Some(&workspace.to_string()),
                &[],
            )
            .await
            .unwrap();
        }
    }
    let rows: Vec<_> = [first, second]
        .into_iter()
        .map(|(tenant, workspace)| RelationshipSinkRow {
            source_name: names[0].clone(),
            target_name: names[1].clone(),
            relation_type: "KNOWS".into(),
            description: "scope regression".into(),
            weight: 1.0,
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
        })
        .collect();
    assert!(matches!(
        sink.upsert_relationships_batch(&rows).await,
        Err(edgequake_pipeline::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidInput(_)
        ))
    ));
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM relationships WHERE workspace_id = ANY($1)")
            .bind(vec![first.1, second.1])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "reject before any relationship side effect");
    let report = sink.upsert_relationships_batch(&rows[..1]).await.unwrap();
    assert_eq!(report.ids.len(), 1);
    let row = &rows[0];
    sink.upsert_relationship(
        &row.source_name,
        &row.target_name,
        &row.relation_type,
        "single lookup",
        row.weight,
        row.tenant_id.as_deref(),
        row.workspace_id.as_deref(),
    )
    .await
    .unwrap();
    let scopes: Vec<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM relationships WHERE workspace_id = ANY($1)")
            .bind(vec![first.1, second.1])
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(scopes, vec![first.1]);
    assert!(matches!(
        sink.remove_entity_sources(&names[0], Some("invalid-uuid"), &[], &[])
            .await,
        Err(edgequake_pipeline::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidInput(_)
        ))
    ));
    assert!(matches!(
        sink.upsert_entity(
            &names[0],
            "PERSON",
            "invalid scope",
            Some("invalid-uuid"),
            None,
            &[]
        )
        .await,
        Err(edgequake_pipeline::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidInput(_)
        ))
    ));
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM entities WHERE name=ANY($1)")
        .bind(&names[..])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 4);
    sqlx::query("DELETE FROM relationships WHERE workspace_id=ANY($1)")
        .bind(vec![first.1, second.1])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM entities WHERE name=ANY($1)")
        .bind(&names[..])
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
async fn workspace_embedding_count_timeout_is_an_error_and_recovers() {
    use edgequake_core::{WorkspaceService, WorkspaceServiceImpl};
    use std::time::Duration;
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let (_, workspace) = scope(&pool).await;
    let query_pool = edgequake_storage::adapters::postgres::with_session_hygiene_labeled(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(2))
            .before_acquire(|conn, _| {
                Box::pin(async move {
                    sqlx::query("SELECT set_config('statement_timeout', '200ms', false)")
                        .execute(conn)
                        .await?;
                    Ok(true)
                })
            }),
        "edgequake:query",
    )
    .connect_with((*pool.connect_options()).clone())
    .await
    .unwrap();
    let service = WorkspaceServiceImpl::new(query_pool.clone());
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE public.chunk_embeddings IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        service.get_workspace_stats(workspace),
    )
    .await;
    lock.rollback().await.unwrap();
    let error = result
        .expect("statistics query must remain bounded")
        .expect_err("a database timeout must not become a valid zero count");
    assert!(
        error
            .to_string()
            .contains("Failed to get workspace embedding count"),
        "{error}"
    );
    assert_eq!(
        service
            .get_workspace_stats(workspace)
            .await
            .unwrap()
            .embedding_count,
        0
    );
    query_pool.close().await;
    pool.close().await;
}
