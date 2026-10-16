//! A tenant's connection list includes shared rows and skips another tenant.
#![cfg(feature = "postgres")]

#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use edgequake_api::handlers::connections::connection_list_sql;
use sqlx::Row;
use uuid::Uuid;

async fn insert_connection(pool: &sqlx::PgPool, tenant: Option<Uuid>, slug: &str) {
    sqlx::query(
        "INSERT INTO provider_connections (tenant_id, slug, display_name, api_shape, locality, base_url)
         VALUES ($1, $2, $2, 'openai_chat', 'local', 'http://127.0.0.1')",
    )
    .bind(tenant)
    .bind(slug)
    .execute(pool)
    .await
    .expect("connection");
}

#[tokio::test]
async fn tenant_connection_list_keeps_shared_rows_and_skips_the_other_tenant() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let own = format!("own-{tenant_a}");
    let other = format!("other-{tenant_b}");
    let shared = format!("shared-{tenant_a}");
    insert_connection(&pool, Some(tenant_a), &own).await;
    insert_connection(&pool, Some(tenant_b), &other).await;
    insert_connection(&pool, None, &shared).await;

    let scoped = sqlx::query(&connection_list_sql(true))
        .bind(tenant_a)
        .fetch_all(&pool)
        .await
        .expect("scoped list");
    let slugs: Vec<String> = scoped.iter().map(|row| row.get("slug")).collect();
    assert!(slugs.contains(&own));
    assert!(slugs.contains(&shared));
    assert!(!slugs.contains(&other));

    let open = sqlx::query(&connection_list_sql(false))
        .fetch_all(&pool)
        .await
        .expect("open list");
    let open_slugs: Vec<String> = open.iter().map(|row| row.get("slug")).collect();
    assert!(open_slugs.contains(&own));
    assert!(open_slugs.contains(&other));
    assert!(open_slugs.contains(&shared));

    sqlx::query("DELETE FROM provider_connections WHERE slug = ANY($1)")
        .bind(vec![own, other, shared])
        .execute(&pool)
        .await
        .expect("cleanup");
}
