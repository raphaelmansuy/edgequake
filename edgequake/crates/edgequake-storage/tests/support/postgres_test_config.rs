//! Shared PostgreSQL test configuration (STORE-DRY-003 / P2-11).
#![allow(dead_code)]
//!
//! Single source for contract and e2e tests that need an isolated namespace.

use edgequake_storage::PostgresConfig;
use std::env;
use std::time::Duration;
use uuid::Uuid;

/// Embedded migrations (SSOT: `edgequake/migrations`). Used to auto-provision
/// the dedicated scratch test database so tests never touch the shared dev DB.
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Name of the dedicated scratch test database.
///
/// WHY (root cause of "many documents with the same title" in the dev UI):
/// tests historically resolved `DATABASE_URL` / `/tmp/edgequake-db-url`, both of
/// which point at the **shared dev database**. KV/vector tables get a per-test
/// random namespace, but post-SPEC-091 `public.documents` is a single global
/// typed table — so every document-writing e2e test leaked rows into the dev
/// document store (e.g. 1600 `Wipe Scale` rows, repeated `Tech Article PG`).
/// Routing every test to an isolated `{dev}_test` database makes that pollution
/// impossible while preserving the exact already-migrated schema tests expect.
/// Override with `EDGEQUAKE_TEST_DATABASE` (e.g. CI pointing at another cluster).
fn test_database_name(base: &str) -> String {
    if let Ok(over) = env::var("EDGEQUAKE_TEST_DATABASE") {
        if !over.trim().is_empty() {
            return over.trim().to_string();
        }
    }
    // Idempotent: a base already suffixed `_test` is left as-is.
    if base.ends_with("_test") {
        base.to_string()
    } else {
        format!("{base}_test")
    }
}

/// Soft-skip unless `EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1` (SPEC-060: nightly hard gate).
pub fn require_or_skip_postgres(namespace_prefix: &str) -> Option<PostgresConfig> {
    if let Some(cfg) = contract_postgres_config(namespace_prefix) {
        return Some(cfg);
    }
    let strict = env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if strict {
        panic!(
            "EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but DATABASE_URL/POSTGRES_PASSWORD missing \
             (also checked /tmp/edgequake-db-url)"
        );
    }
    eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
    None
}

/// Like [`require_or_skip_postgres`], but forces an exact namespace string.
///
/// Used by process-kill parent/child pairs that must share one AGE graph.
pub fn require_or_skip_postgres_exact(namespace: &str) -> Option<PostgresConfig> {
    let Some(mut cfg) = contract_postgres_config("spec149_kill") else {
        let strict = env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if strict {
            panic!(
                "EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but DATABASE_URL/POSTGRES_PASSWORD missing \
                 (also checked /tmp/edgequake-db-url)"
            );
        }
        eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
        return None;
    };
    cfg.namespace = namespace.to_string();
    Some(cfg)
}

/// Build a postgres config when `DATABASE_URL` or `POSTGRES_PASSWORD` is set; otherwise `None`.
///
/// The resolved database is redirected to a dedicated scratch test database
/// (see [`test_database_name`]) and auto-provisioned once per process (see
/// [`ensure_test_db_ready`]) so tests run fully isolated from the dev database.
pub fn contract_postgres_config(namespace_prefix: &str) -> Option<PostgresConfig> {
    let cfg = resolve_postgres_config(namespace_prefix)?;
    ensure_test_db_ready(&cfg);
    Some(cfg)
}

fn resolve_postgres_config(namespace_prefix: &str) -> Option<PostgresConfig> {
    if let Ok(url) = env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return postgres_config_from_database_url(url.trim(), namespace_prefix);
        }
    }
    // make-dev writes the effective URL here (password may differ from defaults).
    if let Ok(url) = std::fs::read_to_string("/tmp/edgequake-db-url") {
        let url = url.trim();
        if !url.is_empty() {
            return postgres_config_from_database_url(url, namespace_prefix);
        }
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
        database: test_database_name(&database),
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
    let base_db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
    PostgresConfig {
        host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("POSTGRES_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5432),
        database: test_database_name(&base_db),
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

/// Provision the scratch test database once per test process: create it when
/// missing, then apply all embedded migrations so tests see an already-migrated
/// schema identical to (but isolated from) the dev database.
///
/// Best-effort: when the server is unreachable the caller still returns a
/// config and the test soft-skips / fails on connect exactly as it would
/// against the dev database today. Runs on a dedicated thread + runtime so it
/// is safe to call from inside an async test (no nested-runtime panic).
fn ensure_test_db_ready(cfg: &PostgresConfig) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let cfg = cfg.clone();
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test-db provision runtime");
            rt.block_on(provision_test_db(cfg));
        })
        .join();
    });
}

async fn provision_test_db(cfg: PostgresConfig) {
    let admin_url = format!(
        "postgres://{}:{}@{}:{}/postgres",
        cfg.user, cfg.password, cfg.host, cfg.port
    );
    let Ok(admin) = sqlx::PgPool::connect(&admin_url).await else {
        return;
    };
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(&cfg.database)
            .fetch_one(&admin)
            .await
            .unwrap_or(false);
    if !exists {
        // CREATE DATABASE cannot be parameterized; the name is derived internally
        // (`{base}_test` or the `EDGEQUAKE_TEST_DATABASE` override), never user SQL.
        let _ = sqlx::query(&format!("CREATE DATABASE {}", cfg.database))
            .execute(&admin)
            .await;
    }
    admin.close().await;

    let test_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        cfg.user, cfg.password, cfg.host, cfg.port, cfg.database
    );
    if let Ok(pool) = sqlx::PgPool::connect(&test_url).await {
        // Scratch DB only: reconcile known expandable-migration checksum
        // drifts (SPEC-110/111) so sqlx migrate can apply pending files
        // without requiring EDGEQUAKE_DEV_MODE (prod still fails closed).
        repair_test_db_migration_checksums(&pool).await;
        repair_stale_spec149_migration_collision(&pool).await;
        // Idempotent: applies only pending migrations, so concurrent test
        // processes and repeat runs converge without dropping anything.
        if let Err(e) = MIGRATOR.run(&pool).await {
            eprintln!("test-db provisioning migrate failed: {e}");
        }
        pool.close().await;
    }
}

/// Scratch DBs that recorded a non-PROVIDER-ACCESS version 150 leave the
/// ledger tables missing while sqlx thinks 150 is done. Drop the orphan row
/// (and any later orphans) so the real 150–153 files can apply.
async fn repair_stale_spec149_migration_collision(pool: &sqlx::PgPool) {
    let Ok(ledger_present): Result<bool, _> =
        sqlx::query_scalar("SELECT to_regclass('public.data_bindings') IS NOT NULL")
            .fetch_one(pool)
            .await
    else {
        return;
    };
    if ledger_present {
        return;
    }
    let Ok(stale_150): Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT description FROM _sqlx_migrations \
         WHERE version = 150 AND success = true",
    )
    .fetch_optional(pool)
    .await
    else {
        return;
    };
    let Some(description) = stale_150 else {
        return;
    };
    if description == "provider access ledger" {
        return;
    }
    eprintln!("test-db: clearing stale migration 150 ({description}) so SPEC-149 ledger can apply");
    let _ = sqlx::query("DELETE FROM _sqlx_migrations WHERE version >= 150")
        .execute(pool)
        .await;
}

/// Update `_sqlx_migrations.checksum` for known fossils so the isolated
/// `{db}_test` scratch database can continue after in-place migration edits.
/// Fossils (incl. `dev_only`) come from `edgequake/migrations/manifest.toml`;
/// current hashes come from `checksums.lock`.
async fn repair_test_db_migration_checksums(pool: &sqlx::PgPool) {
    let Ok(exists): Result<bool, _> =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await
    else {
        return;
    };
    if !exists {
        return;
    }

    let current_by_version = load_checksums_lock_by_version();
    let manifest = edgequake_migrate_manifest::load();
    for entry in &manifest.migration {
        let Some(fixed) = current_by_version.get(&entry.version) else {
            continue;
        };
        for fossil in &entry.fossils {
            let _ = sqlx::query(
                "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') \
                 WHERE version = $2 AND success = true \
                   AND encode(checksum, 'hex') = $3",
            )
            .bind(fixed.as_str())
            .bind(entry.version)
            .bind(fossil.sha384.as_str())
            .execute(pool)
            .await;
        }
    }
}

fn load_checksums_lock_by_version() -> std::collections::HashMap<i64, String> {
    let lock = include_str!("../../../../migrations/checksums.lock");
    let mut map = std::collections::HashMap::new();
    for line in lock.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { continue };
        let Some(file) = parts.next() else { continue };
        let digits: String = file.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            map.insert(v, hash.to_ascii_lowercase());
        }
    }
    map
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
