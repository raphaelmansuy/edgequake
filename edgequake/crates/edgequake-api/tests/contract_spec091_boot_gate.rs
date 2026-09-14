//! SPEC-091 Doc 17 (LD-15, LAW-B1..B5) — boot migration gating contract.
//!
//! Proves the serving boot gate is fail-closed and actionable:
//! - fresh / behind databases refuse with the pinned exit-78 message
//!   (pending count + `edgequake migrate dry-run` + `edgequake migrate` + runbook);
//! - a database newer than the binary refuses (downgrade protection, LAW-B5);
//! - after the CLI-owned apply, the same database passes the gate with no flag;
//! - a stale `EDGEQUAKE_ALLOW_BOOT_MIGRATE=1` warns but changes nothing
//!   (schema untouched);
//! - `/health`'s drift derivation (`schema_drift`) agrees with the gate (DRY).
//!
//! Each test runs against its own scratch database (created fresh, dropped on
//! completion) so ledger states never leak between cases. Skips cleanly when
//! no database is reachable.

#![cfg(feature = "postgres")]

use serial_test::serial;
use sqlx::PgPool;

use edgequake_api::state::migration_bootstrap::{
    boot_gate_downgrade_message, boot_gate_pending_message, bootstrap_for_serving, schema_drift,
    warn_if_removed_boot_flag_set, SchemaDrift, BOOT_GATE_EXIT_CODE, BOOT_GATE_REFUSAL_PREFIX,
};

/// Test-side embedded migrator (SSOT: `edgequake/migrations`) — used to
/// enumerate embedded versions and to play the CLI's apply role in the
/// succeed-after-migrate case.
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

const RUNBOOK_PATH: &str = "docs/operations/spec091-upgrade-from-v0.22.0.md";

/// Resolve the base URL from env; `None` → caller skips.
fn base_url() -> Option<String> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return Some(url);
        }
    }
    let password = std::env::var("POSTGRES_PASSWORD").ok()?;
    let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
    let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
    Some(format!("postgresql://{user}:{password}@{host}:{port}/{db}"))
}

/// Rewrite the database segment of a connection URL (mirrors `test_db.rs`).
fn with_database(url: &str, db: &str) -> String {
    let (head, query) = match url.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (url, None),
    };
    let idx = head.rfind('/').expect("URL has a database segment");
    let (prefix, _) = head.split_at(idx + 1);
    let rewritten = format!("{prefix}{db}");
    match query {
        Some(q) => format!("{rewritten}?{q}"),
        None => rewritten,
    }
}

/// Create a uniquely-named scratch database; returns its URL. Best-effort:
/// `None` when the server is unreachable (caller skips).
async fn create_scratch_db(base: &str, label: &str) -> Option<(String, String)> {
    let admin_url = with_database(base, "postgres");
    let admin = PgPool::connect(&admin_url).await.ok()?;
    let name = format!(
        "eq_bootgate_{}_{}",
        label,
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .ok()?;
    admin.close().await;
    Some((with_database(base, &name), name))
}

async fn drop_scratch_db(base: &str, name: &str) {
    let admin_url = with_database(base, "postgres");
    if let Ok(admin) = PgPool::connect(&admin_url).await {
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
            .execute(&admin)
            .await;
        admin.close().await;
    }
}

/// sqlx's own ledger DDL (kept in sync with sqlx 0.8 `MIGRATOR`).
async fn create_fake_ledger(pool: &PgPool) {
    sqlx::query(
        "CREATE TABLE _sqlx_migrations (\
            version BIGINT PRIMARY KEY,\
            description TEXT NOT NULL,\
            installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),\
            success BOOLEAN NOT NULL,\
            checksum BYTEA NOT NULL,\
            execution_time BIGINT NOT NULL\
        )",
    )
    .execute(pool)
    .await
    .expect("create fake ledger");
}

async fn seed_fake_applied(pool: &PgPool, versions: &[i64]) {
    for v in versions {
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
             VALUES ($1, 'contract-seed', true, '\\x00', 1)",
        )
        .bind(v)
        .execute(pool)
        .await
        .expect("seed fake applied version");
    }
}

fn embedded_versions() -> Vec<i64> {
    MIGRATOR.migrations.iter().map(|m| m.version).collect()
}

/// Assert the pinned refusal contract (LAW-B3 / R-B4): every element the
/// operator needs to reach the preview in one step.
fn assert_pending_message_pins(message: &str, expected_pending: &[i64]) {
    assert!(
        message.contains(BOOT_GATE_REFUSAL_PREFIX),
        "message must carry the exit-78 sentinel prefix: {message}"
    );
    assert!(
        message.contains(&format!("{} pending migration(s)", expected_pending.len())),
        "message must name the pending count: {message}"
    );
    for v in expected_pending {
        assert!(
            message.contains(&v.to_string()),
            "message must list pending version {v}: {message}"
        );
    }
    assert!(
        message.contains("edgequake migrate dry-run"),
        "message must teach the preview (LAW-B3): {message}"
    );
    assert!(
        message.contains("edgequake migrate"),
        "message must teach the apply command: {message}"
    );
    assert!(
        message.contains(RUNBOOK_PATH),
        "message must point at the runbook: {message}"
    );
}

#[tokio::test]
async fn fresh_db_refuses_with_actionable_message() {
    let Some(base) = base_url() else {
        eprintln!("skipping: no database configured");
        return;
    };
    let Some((url, name)) = create_scratch_db(&base, "fresh").await else {
        eprintln!("skipping: cannot create scratch db");
        return;
    };

    let pool = PgPool::connect(&url).await.expect("connect scratch");
    let err = bootstrap_for_serving(&pool)
        .await
        .expect_err("fresh DB must refuse (EC-B1)");
    let message = err.to_string();

    assert_pending_message_pins(&message, &embedded_versions());
    assert_eq!(BOOT_GATE_EXIT_CODE, 78, "exit code is EX_CONFIG (78)");

    // Nothing was applied: the ledger must not even exist.
    let ledger_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = '_sqlx_migrations')",
    )
    .fetch_one(&pool)
    .await
    .expect("ledger probe");
    assert!(!ledger_exists, "refusal must never mutate schema (LAW-B1)");

    pool.close().await;
    drop_scratch_db(&base, &name).await;
}

#[tokio::test]
async fn behind_db_refuses_lists_pending() {
    let Some(base) = base_url() else {
        eprintln!("skipping: no database configured");
        return;
    };
    let Some((url, name)) = create_scratch_db(&base, "behind").await else {
        eprintln!("skipping: cannot create scratch db");
        return;
    };

    // Claim every embedded version applied EXCEPT the last two — pending must
    // be exactly those two (EC-B2).
    let versions = embedded_versions();
    let pending_expected = &versions[versions.len() - 2..];
    let pool = PgPool::connect(&url).await.expect("connect scratch");
    create_fake_ledger(&pool).await;
    seed_fake_applied(&pool, &versions[..versions.len() - 2]).await;

    let err = bootstrap_for_serving(&pool)
        .await
        .expect_err("behind DB must refuse");
    let message = err.to_string();
    assert_pending_message_pins(&message, pending_expected);

    // Drift derivation agrees with the gate (LAW-B3 — one computation).
    let drift = schema_drift(&pool).await.expect("drift derivable");
    assert_eq!(drift.pending_count, 2);
    assert!(!drift.db_newer_than_binary);
    assert!(drift.migration_required());

    pool.close().await;
    drop_scratch_db(&base, &name).await;
}

#[tokio::test]
async fn newer_db_refuses_downgrade() {
    let Some(base) = base_url() else {
        eprintln!("skipping: no database configured");
        return;
    };
    let Some((url, name)) = create_scratch_db(&base, "newer").await else {
        eprintln!("skipping: cannot create scratch db");
        return;
    };

    let embedded_max = embedded_versions().into_iter().max().unwrap();
    let newer = embedded_max + 1000;
    let pool = PgPool::connect(&url).await.expect("connect scratch");
    create_fake_ledger(&pool).await;
    seed_fake_applied(&pool, &[newer]).await;

    let err = bootstrap_for_serving(&pool)
        .await
        .expect_err("newer DB must refuse (EC-B3 / LAW-B5)");
    let message = err.to_string();
    assert!(message.contains(BOOT_GATE_REFUSAL_PREFIX), "{message}");
    assert!(
        message.contains("NEWER than this binary"),
        "distinct downgrade message: {message}"
    );
    assert!(message.contains(&format!("v{newer}")), "{message}");
    assert!(message.contains(&format!("v{embedded_max}")), "{message}");

    // Drift marks the disagreement as operator-action-required too.
    let drift = schema_drift(&pool).await.expect("drift derivable");
    assert!(drift.db_newer_than_binary);
    assert!(drift.migration_required());
    assert_eq!(drift.applied_max, newer);
    assert_eq!(drift.embedded_max, embedded_max);

    // The downgrade message builder itself is pinned (single builder, DRY).
    let pinned = boot_gate_downgrade_message(newer, embedded_max);
    assert!(pinned.contains(BOOT_GATE_REFUSAL_PREFIX));

    pool.close().await;
    drop_scratch_db(&base, &name).await;
}

#[tokio::test]
async fn boot_succeeds_after_cli_owned_migrate() {
    let Some(base) = base_url() else {
        eprintln!("skipping: no database configured");
        return;
    };
    let Some((url, name)) = create_scratch_db(&base, "full").await else {
        eprintln!("skipping: cannot create scratch db");
        return;
    };

    let pool = PgPool::connect(&url).await.expect("connect scratch");

    // Phase 1: refuse (EC-B1).
    let err = bootstrap_for_serving(&pool)
        .await
        .expect_err("fresh DB must refuse before migrate");
    assert!(err.to_string().contains(BOOT_GATE_REFUSAL_PREFIX));

    // Phase 2: the CLI-owned apply (played by the same embedded migrator the
    // CLI drives via `run_postgres_migrations`) — no flags involved.
    MIGRATOR.run(&pool).await.expect("apply full migration set");

    // Phase 3: gate passes with zero flags set (LAW-B1 — only the apply moved
    // the schema; boot merely verifies).
    let report = bootstrap_for_serving(&pool)
        .await
        .expect("boot must pass once schema matches the binary");
    assert_eq!(report.pending_before, 0);

    let drift = schema_drift(&pool).await.expect("drift derivable");
    assert_eq!(drift.pending_count, 0);
    assert!(!drift.migration_required());

    pool.close().await;
    drop_scratch_db(&base, &name).await;
}

#[tokio::test]
#[serial]
async fn stale_flag_warns_and_gate_still_refuses() {
    let Some(base) = base_url() else {
        eprintln!("skipping: no database configured");
        return;
    };
    let Some((url, name)) = create_scratch_db(&base, "stale").await else {
        eprintln!("skipping: cannot create scratch db");
        return;
    };

    // EC-B9: the removed flag must change NOTHING — the gate stays fail-closed.
    std::env::set_var("EDGEQUAKE_ALLOW_BOOT_MIGRATE", "1");
    warn_if_removed_boot_flag_set(); // must not panic; emits the one WARN

    let pool = PgPool::connect(&url).await.expect("connect scratch");
    let err = bootstrap_for_serving(&pool)
        .await
        .expect_err("stale flag must not reopen the gate");
    assert!(err.to_string().contains(BOOT_GATE_REFUSAL_PREFIX));

    // Schema untouched: no ledger created.
    let ledger_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = '_sqlx_migrations')",
    )
    .fetch_one(&pool)
    .await
    .expect("ledger probe");
    assert!(!ledger_exists, "stale flag must not apply schema (EC-B9)");

    std::env::remove_var("EDGEQUAKE_ALLOW_BOOT_MIGRATE");
    pool.close().await;
    drop_scratch_db(&base, &name).await;
}

#[test]
fn message_builder_pins_and_drift_logic() {
    // LAW-B3: one builder serves both refusal sites — pin its contract here so
    // message drift fails loudly (R-B4).
    let pending = boot_gate_pending_message(&[106, 125]);
    assert_pending_message_pins(&pending, &[106, 125]);

    let downgrade = boot_gate_downgrade_message(200, 126);
    assert!(downgrade.contains(BOOT_GATE_REFUSAL_PREFIX));
    assert!(downgrade.contains("NEWER than this binary"));

    // SchemaDrift logic (no DB needed). Use embedded_max+1 — never hardcode a
    // foreign-branch version (e.g. SPEC-146's 150) into this tree's fixtures.
    let embedded_max = MIGRATOR
        .migrations
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0);
    assert!(SchemaDrift {
        pending_count: 1,
        db_newer_than_binary: false,
        applied_max: embedded_max.saturating_sub(1),
        embedded_max,
    }
    .migration_required());
    assert!(SchemaDrift {
        pending_count: 0,
        db_newer_than_binary: true,
        applied_max: embedded_max + 1,
        embedded_max,
    }
    .migration_required());
    assert!(!SchemaDrift {
        pending_count: 0,
        db_newer_than_binary: false,
        applied_max: embedded_max,
        embedded_max,
    }
    .migration_required());
}

#[tokio::test]
#[serial]
async fn irreversible_only_pending_soft_allows_boot() {
    use edgequake_api::state::migration_bootstrap::{
        is_irreversible_drop, pending_only_irreversible_drops,
    };

    let Some(base) = base_url() else {
        eprintln!("skipping: no database configured");
        return;
    };
    let Some((url, name)) = create_scratch_db(&base, "irrev").await else {
        eprintln!("skipping: cannot create scratch db");
        return;
    };

    let versions = embedded_versions();
    let pending_only: Vec<i64> = versions
        .iter()
        .copied()
        .filter(|v| is_irreversible_drop(*v))
        .collect();
    assert!(
        pending_only_irreversible_drops(&[131]),
        "131 alone is irreversible-only"
    );
    // Seed every embedded version except 131 — only fleet drop pending.
    let applied: Vec<i64> = versions.iter().copied().filter(|v| *v != 131).collect();
    let pool = PgPool::connect(&url).await.expect("connect scratch");
    create_fake_ledger(&pool).await;
    seed_fake_applied(&pool, &applied).await;

    bootstrap_for_serving(&pool)
        .await
        .expect("irreversible-only pending must soft-allow serving boot");

    let drift = schema_drift(&pool).await.expect("drift");
    assert_eq!(drift.pending_count, 1);
    assert!(drift.migration_required());

    pool.close().await;
    drop_scratch_db(&base, &name).await;
    let _ = pending_only; // silence if unused on older embeds
}
