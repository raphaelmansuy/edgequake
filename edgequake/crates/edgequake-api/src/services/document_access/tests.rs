//! Live compatibility checks; run serially after the browser suite.
use super::*;

#[tokio::test]
#[ignore = "requires the disposable SPEC-180 database and seed.sql"]
async fn enforcement_off_preserves_legacy_uploaderless_admission() {
    let url = std::env::var("SPEC180_DATABASE_URL").expect("SPEC180_DATABASE_URL is required");
    assert_eq!(url.rsplit('/').next(), Some("edgequake_spec180"));
    let pool = PgPool::connect(&url).await.unwrap();
    let workspace = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
    let id = Uuid::new_v4();
    sqlx::query("UPDATE public.workspace_clearance_settings SET enforcement=false,policy_epoch=policy_epoch+1 WHERE workspace_id=$1")
        .bind(workspace).execute(&pool).await.unwrap();
    let result = admit_initial_access(
        &pool,
        &TenantContext {
            tenant_id: Some("00000000-0000-0000-0000-000000000002".into()),
            workspace_id: Some(workspace.to_string()),
            user_id: None,
        },
        id,
        "SPEC180 legacy compatibility",
        "spec180-legacy",
        None,
    )
    .await;
    let saved: Option<(Option<i16>, i64)> = sqlx::query_as(
        "SELECT classification_rank,label_version FROM public.documents WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    // Restore before assertions to keep a failed test from changing the fixture policy.
    sqlx::query("DELETE FROM public.documents WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE public.workspace_clearance_settings SET enforcement=true,policy_epoch=policy_epoch+1 WHERE workspace_id=$1")
        .bind(workspace).execute(&pool).await.unwrap();
    assert!(result.is_ok(), "legacy admission failed: {result:?}");
    assert_eq!(saved, Some((None, 0)));
    pool.close().await;
}
