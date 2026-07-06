//! Shared PostgreSQL test configuration (STORE-DRY-003 / P2-11).
#![allow(dead_code)]
//!
//! Single source for contract and e2e tests that need an isolated namespace.

use edgequake_storage::PostgresConfig;
use std::env;
use std::time::Duration;
use uuid::Uuid;

/// Build a postgres config when `DATABASE_URL` or `POSTGRES_PASSWORD` is set; otherwise `None`.
pub fn contract_postgres_config(namespace_prefix: &str) -> Option<PostgresConfig> {
    if let Ok(url) = env::var("DATABASE_URL") {
        return postgres_config_from_database_url(&url, namespace_prefix);
    }

    let password = env::var("POSTGRES_PASSWORD").ok()?;
    Some(postgres_config_from_env(password, namespace_prefix))
}

fn postgres_config_from_database_url(url: &str, namespace_prefix: &str) -> Option<PostgresConfig> {
    let without_scheme = url.split("://").nth(1)?;
    let (auth, host_path) = without_scheme.split_once('@')?;
    let (user, password) = auth.split_once(':')?;
    let (host_port, db_path) = host_path.split_once('/')?;
    let database = db_path.split('?').next()?.to_string();
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().ok()?),
        None => (host_port.to_string(), 5432),
    };

    Some(PostgresConfig {
        host,
        port,
        database,
        user: user.to_string(),
        password: password.to_string(),
        namespace: isolated_namespace(namespace_prefix),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(60),
        ..Default::default()
    })
}

fn postgres_config_from_env(password: String, namespace_prefix: &str) -> PostgresConfig {
    PostgresConfig {
        host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("POSTGRES_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5432),
        database: env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
        user: env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
        password,
        namespace: isolated_namespace(namespace_prefix),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(60),
        ..Default::default()
    }
}

fn isolated_namespace(namespace_prefix: &str) -> String {
    format!(
        "{}_{}",
        namespace_prefix,
        &Uuid::new_v4().to_string().replace('-', "")[..8]
    )
}

/// Connection pool for contract tests (DRY URL builder).
pub async fn contract_pg_pool(config: &PostgresConfig) -> sqlx::PgPool {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.user, config.password, config.host, config.port, config.database
    );
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .connect(&database_url)
        .await
        .expect("postgres pool")
}

/// Seed tenant + user so conversation/folder FK constraints pass (mirrors API user bootstrap).
pub async fn seed_tenant_and_user(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    let slug = format!("spec017_{}", tenant_id.as_simple());
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, name, slug, is_active, settings, metadata)
        VALUES ($1, 'SPEC-017 contract tenant', $2, TRUE, '{}', '{}')
        ON CONFLICT (tenant_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(&slug)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users (user_id, tenant_id, username, email, password_hash, role, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'anonymous', 'user', TRUE, NOW(), NOW())
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(format!("user_{}", &user_id.to_string()[..8]))
    .bind(format!("{}@spec017.local", &user_id.to_string()[..8]))
    .execute(pool)
    .await?;

    Ok(())
}
