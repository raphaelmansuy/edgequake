//! Regression: workspace vector stats triggers must initialize when pooled
//! connections have graph `search_path` (ag_catalog) pollution.
//!
//! Run: `cargo test -p edgequake-storage --features postgres --test postgres_workspace_vector_stats`

#![cfg(feature = "postgres")]

use std::env;
use std::sync::Arc;

use edgequake_storage::adapters::postgres::{
    PgVectorStorage, PgWorkspaceVectorRegistry, PostgresConfig, PostgresPool,
};
use edgequake_storage::traits::{VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

fn test_config(namespace: &str) -> Option<PostgresConfig> {
    if let Ok(url) = env::var("DATABASE_URL") {
        let without_scheme = url.split("://").nth(1)?;
        let (auth, host_path) = without_scheme.split_once('@')?;
        let (user, password) = auth.split_once(':')?;
        let (host_port, db_path) = host_path.split_once('/')?;
        let db = db_path.split('?').next().unwrap_or(db_path).to_string();
        let (host, port) = match host_port.split_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().ok()?),
            None => (host_port.to_string(), 5432),
        };
        return Some(
            PostgresConfig::new(host, port, db, user.to_string(), password.to_string())
                .with_namespace(namespace),
        );
    }

    let password = env::var("POSTGRES_PASSWORD").ok()?;
    Some(
        PostgresConfig::new(
            env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            env::var("POSTGRES_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432),
            env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
            env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
            password,
        )
        .with_namespace(namespace),
    )
}

async fn shared_pool(config: &PostgresConfig) -> sqlx::PgPool {
    let url = config.connection_url();
    PgPoolOptions::new()
        .max_connections(5)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET search_path TO public")
                    .execute(conn)
                    .await
                    .map(|_| ())
            })
        })
        .connect(&url)
        .await
        .expect("postgres pool")
}

/// Simulate graph leaving a connection on `ag_catalog`-only search_path, then
/// create workspace vector storage (must not fail with missing stats function).
#[tokio::test]
async fn workspace_vector_stats_init_with_search_path_pollution() {
    let Some(base_config) = test_config("default") else {
        let strict = env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if strict {
            panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but DATABASE_URL/POSTGRES_PASSWORD missing");
        }
        eprintln!("SKIP: DATABASE_URL or POSTGRES_PASSWORD not set");
        return;
    };

    let workspace_id = Uuid::new_v4();
    let short_id = &workspace_id.to_string()[..8];
    let ws_namespace = format!("default_ws_{short_id}");
    let prefix = format!("eq_{}", ws_namespace.replace('-', "_"));
    let polluted_fn = format!("eq_{prefix}_vectors_stats_insert");

    let pool = shared_pool(&base_config).await;
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&pool)
        .await
        .expect("pgvector extension");
    // CI uses pgvector-only Postgres (no Apache AGE); create ag_catalog so we can
    // simulate graph search_path pollution without the full AGE extension.
    sqlx::query("CREATE SCHEMA IF NOT EXISTS ag_catalog")
        .execute(&pool)
        .await
        .expect("ag_catalog schema stub");

    let storage_pool = PostgresPool::from_existing(pool.clone(), base_config.clone());

    let default_storage = Arc::new(PgVectorStorage::with_pool_and_dimension(
        storage_pool.clone(),
        base_config.clone(),
        768,
    ));
    default_storage.initialize().await.expect("default init");

    let registry = PgWorkspaceVectorRegistry::new(
        base_config.clone(),
        storage_pool.clone(),
        default_storage,
        768,
    );

    // Pollute one pooled connection: unqualified CREATE lands in ag_catalog.
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SET search_path = ag_catalog")
            .execute(&mut *conn)
            .await
            .expect("set ag_catalog path");
        let bad_fn = format!(
            "CREATE OR REPLACE FUNCTION {polluted_fn}() RETURNS trigger AS $$
             BEGIN RETURN NEW; END;
             $$ LANGUAGE plpgsql"
        );
        sqlx::query(&bad_fn)
            .execute(&mut *conn)
            .await
            .expect("create ag_catalog function");
        // Return connection to pool with public path so vector DDL can run; the stray
        // ag_catalog function simulates graph search_path pollution.
        sqlx::query("SET search_path TO public")
            .execute(&mut *conn)
            .await
            .expect("reset search_path");
    }

    let storage = registry
        .get_or_create(WorkspaceVectorConfig::new(workspace_id, 768))
        .await
        .expect("workspace vector storage must initialize");

    let embedding = vec![0.1_f32; 768];
    storage
        .upsert(&[("vec-1".to_string(), embedding, serde_json::json!({}))])
        .await
        .expect("upsert");

    assert_eq!(storage.count().await.expect("count"), 1);
}
