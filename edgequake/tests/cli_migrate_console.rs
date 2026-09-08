//! SPEC-091 Migration Console — CLI subprocess e2e (waves C0–C3).
//!
//! Exercises the real `edgequake migrate` binary end-to-end: the read-only
//! verbs render (C0), the write verbs are flag-gated (LD-07) and posture-gated
//! (C1 job-control lease refusal, C2 family cutover), and irreversible drops
//! soft-defer without `--confirm-drop` while expandable SAFE SCHEMA still applies (C3).
//!
//! The library-level, multi-phase advisor coverage lives in
//! `edgequake-storage/tests/e2e_spec091_console.rs`; this suite proves the CLI
//! wiring, exit codes, and guardrail surfaces on top of it.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test --test cli_migrate_console -- --nocapture

#![cfg(feature = "postgres")]

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_edgequake");

/// Env vars that could leak from the parent process and change the posture or
/// the gates — always scrubbed, then per-test overrides are applied.
const SCRUB_ENV: &[&str] = &[
    "EDGEQUAKE_MIGRATION_CONSOLE",
    "EDGEQUAKE_MIGRATION_CONFIRM_DROP",
    "EDGEQUAKE_MIGRATION_MODE",
    "EDGEQUAKE_CHUNK_TEXT_AUTHORITY",
    "EDGEQUAKE_KV_FAMILY_METADATA",
    "EDGEQUAKE_KV_FAMILY_WSDOC",
    "EDGEQUAKE_KV_FAMILY_DOC_HASH",
    "EDGEQUAKE_KV_FAMILY_COMPENSATION_QUARANTINE",
    "EDGEQUAKE_KV_FAMILY_CHECKPOINT",
    "EDGEQUAKE_KV_FAMILY_ARTIFACT",
    "EDGEQUAKE_KV_FAMILY_INJECTION",
    "EDGEQUAKE_KV_FAMILY_CACHE",
    "EDGEQUAKE_VECTOR_BACKEND",
];

fn dev_db_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
}

struct CliOut {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

/// Run `edgequake migrate <args>` against `db_url` with a scrubbed environment
/// plus any per-test overrides.
fn run_cli(args: &[&str], db_url: &str, extra_env: &[(&str, &str)]) -> CliOut {
    let mut cmd = Command::new(BIN);
    cmd.arg("migrate")
        .args(args)
        .env("DATABASE_URL", db_url)
        // Keep the logs quiet/deterministic; the posture output is on stdout.
        .env("RUST_LOG", "warn")
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", "");
    for var in SCRUB_ENV {
        cmd.env_remove(var);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn edgequake migrate");
    CliOut {
        status: out.status,
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

// ---------------------------------------------------------------------------
// C0 — read-only verbs render.
// ---------------------------------------------------------------------------

#[test]
fn cli_console_renders_posture_dashboard() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(&["console"], &db, &[]);
    assert!(out.status.success(), "console exit: {}", out.stderr);
    assert!(out.stdout.contains("cutover phase:"), "{}", out.stdout);
    assert!(
        out.stdout.contains("FAMILY"),
        "family table header: {}",
        out.stdout
    );
    assert!(out.stdout.contains("CHUNK"), "family rows: {}", out.stdout);
    // Dev DB is fully migrated → the runbook reaches DONE.
    assert!(out.stdout.contains("DONE"), "{}", out.stdout);
}

#[test]
fn cli_plan_renders_runbook() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(&["plan"], &db, &[]);
    assert!(out.status.success(), "plan exit: {}", out.stderr);
    assert!(out.stdout.contains("NEXT (runbook)"), "{}", out.stdout);
    assert!(out.stdout.contains("DONE"), "{}", out.stdout);
}

#[test]
fn cli_guard_reports_drop_readiness() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(&["guard"], &db, &[]);
    assert!(out.status.success(), "guard exit: {}", out.stderr);
    assert!(out.stdout.contains("GUARD"), "{}", out.stdout);
    assert!(out.stdout.contains("drop-readiness"), "{}", out.stdout);
}

#[test]
fn cli_family_list_covers_all_families() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(&["family", "list"], &db, &[]);
    assert!(out.status.success(), "family list exit: {}", out.stderr);
    for fam in [
        "CHUNK",
        "METADATA",
        "WSDOC",
        "ARTIFACT",
        "INJECTION",
        "CACHE",
    ] {
        assert!(out.stdout.contains(fam), "missing {fam}: {}", out.stdout);
    }
}

// ---------------------------------------------------------------------------
// C1 — job control: LD-07 gate + lease state-machine refusal surfaced.
// ---------------------------------------------------------------------------

#[test]
fn cli_pause_gated_by_ld07() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(&["pause", "w1-chunk-text-backfill"], &db, &[]);
    assert!(
        !out.status.success(),
        "write verb must be refused without LD-07"
    );
    assert!(
        out.stderr.contains("gated (LD-07)"),
        "LD-07 gate reason: {}",
        out.stderr
    );
}

#[test]
fn cli_pause_surfaces_lease_refusal_when_enabled() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    // When the job exists in 'completed', pause is illegal; when the ledger
    // has no row yet (fresh migrate without engine run), find_job_id refuses.
    // Either path must be a clear gate reason — never a silent success.
    let out = run_cli(
        &["pause", "w1-chunk-text-backfill"],
        &db,
        &[("EDGEQUAKE_MIGRATION_CONSOLE", "on")],
    );
    assert!(
        !out.status.success(),
        "pause must fail without a running job"
    );
    assert!(
        out.stderr.contains("in state 'completed'")
            || out.stderr.contains("no migration job found"),
        "lease gate reason: {}",
        out.stderr
    );
}

// ---------------------------------------------------------------------------
// C2 — family cutover: LD-07 gate, posture gate (EC-C1), idempotent no-op.
// ---------------------------------------------------------------------------

#[test]
fn cli_family_set_gated_by_ld07() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(&["family", "set", "CHUNK", "dual"], &db, &[]);
    assert!(!out.status.success());
    assert!(out.stderr.contains("gated (LD-07)"), "{}", out.stderr);
}

#[test]
fn cli_family_set_idempotent_noop_when_enabled() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    // CHUNK is already relational on the dev DB → setting it again is a no-op.
    let out = run_cli(
        &["family", "set", "CHUNK", "relational"],
        &db,
        &[("EDGEQUAKE_MIGRATION_CONSOLE", "on")],
    );
    assert!(out.status.success(), "idempotent set: {}", out.stderr);
    assert!(out.stdout.contains("already relational"), "{}", out.stdout);
}

#[test]
fn cli_family_set_rollback_refused_post_drop() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    // EC-C1 via the CLI: METADATA is relational and the store is dropped, so a
    // rollback to kv must be refused with the 42P01 guardrail reason.
    let out = run_cli(
        &["family", "set", "METADATA", "kv", "--yes"],
        &db,
        &[("EDGEQUAKE_MIGRATION_CONSOLE", "on")],
    );
    assert!(!out.status.success(), "post-drop rollback must be refused");
    assert!(out.stderr.contains("42P01"), "EC-C1 reason: {}", out.stderr);
}

#[test]
fn cli_family_set_unknown_family_rejected() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let out = run_cli(
        &["family", "set", "chunks", "dual"],
        &db,
        &[("EDGEQUAKE_MIGRATION_CONSOLE", "on")],
    );
    assert!(!out.status.success());
    assert!(out.stderr.contains("unknown family"), "{}", out.stderr);
}

// ---------------------------------------------------------------------------
// dry-run — preview only (no schema advance).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cli_migrate_dry_run_preview_does_not_advance_migrations() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_dryrun_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect fresh");
    async fn migrations_max(p: &sqlx::PgPool) -> i64 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
               SELECT 1 FROM information_schema.tables
               WHERE table_schema = 'public' AND table_name = '_sqlx_migrations'
             )",
        )
        .fetch_one(p)
        .await
        .unwrap_or(false);
        if !exists {
            return 0;
        }
        sqlx::query_scalar("SELECT coalesce(max(version), 0) FROM _sqlx_migrations")
            .fetch_one(p)
            .await
            .unwrap_or(0)
    }
    let max_before = migrations_max(&pool).await;

    let out = run_cli(&["dry-run"], &fresh, &[]);
    assert!(
        out.status.success(),
        "dry-run must exit 0 even when drop-readiness is RED: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}\n{}", out.stdout, out.stderr);
    assert!(
        combined.to_ascii_lowercase().contains("dry-run"),
        "must mention dry-run: {combined}"
    );
    assert!(
        combined.contains("125") && combined.to_ascii_uppercase().contains("IRREVERSIBLE"),
        "must call out pending 125 as IRREVERSIBLE: {combined}"
    );

    let max_after = migrations_max(&pool).await;
    assert_eq!(
        max_before, max_after,
        "dry-run must not advance _sqlx_migrations ({max_before} -> {max_after})"
    );
    assert!(
        max_after < 125,
        "dry-run must leave schema pre-drop (max={max_after})"
    );
    pool.close().await;
    drop_db(&db, &name).await;
}

#[test]
fn cli_migrate_usage_mentions_dry_run() {
    let mut cmd = Command::new(BIN);
    cmd.arg("migrate")
        .arg("not-a-real-verb")
        .env("RUST_LOG", "warn")
        .env_remove("DATABASE_URL");
    for var in SCRUB_ENV {
        cmd.env_remove(var);
    }
    let out = cmd.output().expect("spawn");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("dry-run"),
        "usage must mention dry-run: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// C3 — the irreversible drop is refused without --confirm-drop.
// ---------------------------------------------------------------------------

/// Create a fresh, empty database (every migration incl. 125 pending) and
/// return its URL. Uses the maintenance `postgres` database.
async fn create_fresh_db(dev_url: &str, name: &str) -> Option<String> {
    let admin_url = dev_url
        .rsplit_once('/')
        .map(|(base, _)| format!("{base}/postgres"))?;
    let admin = sqlx::PgPool::connect(&admin_url).await.ok()?;
    sqlx::query(&format!("DROP DATABASE IF EXISTS {name}"))
        .execute(&admin)
        .await
        .ok()?;
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .ok()?;
    admin.close().await;
    dev_url
        .rsplit_once('/')
        .map(|(base, _)| format!("{base}/{name}"))
}

async fn drop_db(dev_url: &str, name: &str) {
    if let Some(admin_url) = dev_url
        .rsplit_once('/')
        .map(|(base, _)| format!("{base}/postgres"))
    {
        if let Ok(admin) = sqlx::PgPool::connect(&admin_url).await {
            let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {name}"))
                .execute(&admin)
                .await;
            admin.close().await;
        }
    }
}

/// Apply real schema through `max_version` (CLI-only migrator). Ledger-only
/// seeds cannot exercise expandable apply — SQL needs real objects.
async fn seed_schema_through(db_url: &str, max_version: i64) {
    // SAFETY: process-local env for this test binary only; scrubbed in run_cli.
    unsafe {
        std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    }
    let pool = sqlx::PgPool::connect(db_url).await.expect("connect seed");
    edgequake_api::state::migration_bootstrap::run_postgres_migrations_through(&pool, max_version)
        .await
        .unwrap_or_else(|e| panic!("seed schema through {max_version}: {e}"));
    pool.close().await;
    unsafe {
        std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    }
}

/// C3 — without `--confirm-drop`, expandable SAFE SCHEMA applies and irreversible
/// drops soft-defer (exit 0) with an operator hint pointing at `--confirm-drop`.
#[tokio::test]
async fn cli_migrate_refuses_drop_without_confirm() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_gate_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };
    // Upgrade-shaped: real schema through 124 → 125/126/131 pending irreversibles.
    seed_schema_through(&fresh, 124).await;

    let out = run_cli(&[], &fresh, &[]);
    assert!(
        out.status.success(),
        "expandables must soft-exit 0 while drops defer: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(
        combined.contains("--confirm-drop"),
        "soft-exit must point at --confirm-drop: {combined}"
    );
    assert!(
        combined.contains("dry-run"),
        "soft-exit must hint dry-run preview: {combined}"
    );
    assert!(
        combined.contains("125") || combined.to_ascii_lowercase().contains("irreversible"),
        "must mention deferred irreversible: {combined}"
    );

    drop_db(&db, &name).await;
}

// W4 — migration 126 is surfaced as IRREVERSIBLE and gated behind --confirm-drop.
#[tokio::test]
async fn cli_migrate_marks_126_irreversible_and_refuses_without_confirm() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_vgate_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };
    // Real schema through 125 → 126 (+ later) pending; dry-run must label 126.
    seed_schema_through(&fresh, 125).await;

    let dry = run_cli(&["dry-run"], &fresh, &[]);
    let dry_combined = format!("{}{}", dry.stdout, dry.stderr);
    assert!(
        dry_combined.contains("126") && dry_combined.to_ascii_uppercase().contains("IRREVERSIBLE"),
        "dry-run must call out pending 126 as IRREVERSIBLE: {dry_combined}"
    );

    // No-confirm apply: expandables land; irreversible soft-deferred (exit 0).
    let out = run_cli(&[], &fresh, &[]);
    assert!(
        out.status.success(),
        "expandables must soft-exit 0 with pending irreversible: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stderr.contains("--confirm-drop") || out.stdout.contains("--confirm-drop"),
        "soft-exit must point at --confirm-drop: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );

    drop_db(&db, &name).await;
}

// Doc 17 (LAW-C5 scope) — a FRESH install has nothing legacy to lose: plain
// `migrate` applies the full set (including irreversible 125/126) without
// --confirm-drop. This is the `make dev` cold-start path (EC-B7).
#[tokio::test]
async fn cli_migrate_fresh_install_applies_without_confirm_drop() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_fresh_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };

    let out = run_cli(&[], &fresh, &[]);
    assert!(
        out.status.success(),
        "fresh install must apply without --confirm-drop: {}",
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(
        combined.contains("fresh install"),
        "the fresh-install notice keeps the auto-allow visible: {combined}"
    );

    // The full schema landed — ledger at the embedded latest.
    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect fresh");
    let max_version: i64 =
        sqlx::query_scalar("SELECT coalesce(max(version), 0) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("ledger max");
    assert!(
        max_version >= 131,
        "fresh install reaches the latest migration (incl. fleet drop 131): {max_version}"
    );
    pool.close().await;

    drop_db(&db, &name).await;
}

#[tokio::test]
async fn cli_migrate_no_drop_gate_when_already_applied() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    // Do not depend on the shared DATABASE_URL ledger (local DBs may sit at
    // 126 with pending irreversible 131). Provision a fresh DB, apply the full
    // train once (fresh-install auto-allows irreversibles), then re-run migrate
    // with no --confirm-drop — must succeed with nothing pending.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_nodrop_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };

    let first = run_cli(&[], &fresh, &[]);
    assert!(
        first.status.success(),
        "fresh install seed migrate: {}",
        first.stderr
    );

    let out = run_cli(&[], &fresh, &[]);
    assert!(
        out.status.success(),
        "migrate on a fully-migrated DB: {}",
        out.stderr
    );
    assert!(
        !out.stderr.contains("requires explicit --confirm-drop"),
        "gate must not fire post-drop: {}",
        out.stderr
    );

    drop_db(&db, &name).await;
}

/// Expandables before irreversible 131 apply without `--confirm-drop`; 131 stays pending (soft exit 0).
///
/// First principles: ledger-only seeds cannot exercise partial apply — migration
/// SQL needs real objects. Seed by applying schema through 127, then assert the
/// CLI advances 128–130 and soft-exits on 131.
#[tokio::test]
async fn cli_migrate_applies_expandables_before_fleet_drop_without_confirm() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_exp_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };

    // Real schema through 127 (includes prior irreversible drops on a scratch DB).
    // SAFETY: process-local env for this test binary only; scrubbed in run_cli.
    unsafe {
        std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    }
    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect seed");
    edgequake_api::state::migration_bootstrap::run_postgres_migrations_through(&pool, 127)
        .await
        .expect("seed schema through 127");
    pool.close().await;
    unsafe {
        std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    }

    let out = run_cli(&[], &fresh, &[]);
    assert!(
        out.status.success(),
        "expandables must apply without --confirm-drop: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(
        combined.contains("131") || combined.to_ascii_lowercase().contains("irreversible"),
        "must mention deferred irreversible: {combined}"
    );

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    let applied_130: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 130 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("130");
    let applied_131: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 131 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("131");
    assert!(applied_130, "migration 130 must be applied");
    assert!(!applied_131, "migration 131 must remain pending");
    pool.close().await;

    // Second migrate with only 131 pending: soft-exit 0 (make_dev path).
    let again = run_cli(&[], &fresh, &[]);
    assert!(
        again.status.success(),
        "irreversible-only pending must soft-exit 0: {}",
        again.stderr
    );

    drop_db(&db, &name).await;
}

// ---------------------------------------------------------------------------
// SPEC-137 — 0.25→0.26 mid-cutover honesty (consent alias, abort class, 149).
// ---------------------------------------------------------------------------

/// E2E-137-03: unknown apply flags fail closed (no database required).
#[test]
fn cli_migrate_unknown_apply_flag_rejected() {
    let out = run_cli(
        &["--confirm-drp"],
        "postgres://edgequake:edgequake@127.0.0.1:1/edgequake",
        &[],
    );
    assert!(
        !out.status.success(),
        "unknown flag must be non-zero: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(
        combined.contains("--confirm-drp"),
        "must echo the unknown flag: {combined}"
    );
    assert!(
        combined.contains("--confirm-drop"),
        "must hint canonical consent token: {combined}"
    );
}

#[test]
fn cli_migrate_bare_confirm_is_not_drop_consent() {
    let out = run_cli(
        &["--confirm"],
        "postgres://edgequake:edgequake@127.0.0.1:1/edgequake",
        &[],
    );
    assert!(
        !out.status.success(),
        "stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(combined.contains("--confirm-drop"), "{combined}");
}

/// E2E-137-01: expandable mid-cutover applies 149; DROP OLD stays pending.
#[tokio::test]
async fn cli_migrate_spec137_applies_149_leaves_drops() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_13701_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };

    unsafe {
        std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    }
    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect seed");
    edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(&pool)
        .await
        .expect("expandable seed (omit 125/126/131)");

    let _ = sqlx::query("DROP INDEX IF EXISTS idx_tasks_workspace_document_id")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks DROP COLUMN IF EXISTS document_id")
        .execute(&pool)
        .await;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 149")
        .execute(&pool)
        .await
        .expect("rewind 149");
    pool.close().await;
    unsafe {
        std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    }

    let out = run_cli(&[], &fresh, &[]);
    assert!(
        out.status.success(),
        "expandable migrate must apply 149: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    let applied_149: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 149 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("149");
    let applied_125: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 125 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("125");
    assert!(applied_149, "migration 149 must apply without confirm");
    assert!(!applied_125, "migration 125 must remain pending");
    pool.close().await;

    drop_db(&db, &name).await;
}

/// E2E-137-02 + E2E-137-07: `--drop-confirm` applies empty leftover drops; AGE graphs stay.
#[tokio::test]
async fn cli_migrate_spec137_drop_confirm_alias_and_age_untouched() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_13702_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };
    seed_schema_through(&fresh, 124).await;

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    let graphs_before: Option<i64> =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM ag_catalog.ag_graph")
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
    pool.close().await;

    let out = run_cli(&["--drop-confirm"], &fresh, &[]);
    assert!(
        out.status.success(),
        "--drop-confirm on empty leftover must apply: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect after");
    let applied_125: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 125 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("125");
    let applied_142: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 142 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("142");
    assert!(applied_125, "125 must apply with --drop-confirm");
    assert!(applied_142, "142 must apply after empty leftover drops");

    if let Some(before) = graphs_before {
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM ag_catalog.ag_graph")
            .fetch_one(&pool)
            .await
            .expect("ag_graph after");
        assert_eq!(before, after, "AGE graph count must be unchanged");
    }
    pool.close().await;

    drop_db(&db, &name).await;
}

/// E2E-137-04: Wave D abort class — KV residue, not tasks/pg_locks.
#[tokio::test]
async fn cli_migrate_spec137_wave_d_abort_hint() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_13704_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };
    seed_schema_through(&fresh, 124).await;

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS public.eq_spec137_kv (key TEXT PRIMARY KEY, value JSONB)",
    )
    .execute(&pool)
    .await
    .expect("kv table");
    sqlx::query(
        "INSERT INTO public.eq_spec137_kv (key, value) VALUES \
         ('aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee-chunk-0', '{\"content\":\"orphan\"}'::jsonb)",
    )
    .execute(&pool)
    .await
    .expect("kv residue");
    pool.close().await;

    let out = run_cli(&["--confirm-drop"], &fresh, &[]);
    assert!(
        !out.status.success(),
        "Wave D must fail closed: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(
        combined.contains("Wave D") || combined.contains("un-migrated durable KV"),
        "must name Wave D: {combined}"
    );
    assert!(
        !combined.contains("pg_locks"),
        "must not classify as tasks lock: {combined}"
    );

    drop_db(&db, &name).await;
}

/// E2E-137-05: IW2 abort names provenance-stamp.
#[tokio::test]
async fn cli_migrate_spec137_iw2_abort_hint() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_13705_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };
    unsafe {
        std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    }
    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(&pool)
        .await
        .expect("expandable seed (143 before pending 131)");
    unsafe {
        std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    }
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS public.eq_spec137_vectors \
         (id TEXT PRIMARY KEY, embedding vector, metadata JSONB DEFAULT '{}')",
    )
    .execute(&pool)
    .await;
    sqlx::query(
        "INSERT INTO public.eq_spec137_vectors (id) VALUES ('entity:ORPHAN_SPEC137') \
         ON CONFLICT DO NOTHING",
    )
    .execute(&pool)
    .await
    .expect("uncovered entity vector");
    pool.close().await;

    let out = run_cli(&["--confirm-drop"], &fresh, &[]);
    assert!(
        !out.status.success(),
        "IW2 must fail closed: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    let combined = format!("{}{}", out.stdout, out.stderr).to_ascii_lowercase();
    assert!(
        combined.contains("iw2")
            || combined.contains("provenance")
            || combined.contains("legacy_vector_id"),
        "must name IW2 / provenance: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    assert!(
        combined.contains("provenance-stamp") || combined.contains("iw2-fleet"),
        "hint must name stamp job: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );

    drop_db(&db, &name).await;
}

/// E2E-137-08: `migrate guard` does not mutate `_sqlx_migrations`.
#[tokio::test]
async fn cli_migrate_spec137_guard_does_not_mutate_ledger() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_13708_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };
    let first = run_cli(&[], &fresh, &[]);
    assert!(first.status.success(), "fresh seed: {}", first.stderr);

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    let before_count: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("count before");
    let before_max: i64 =
        sqlx::query_scalar("SELECT coalesce(max(version), 0) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("max before");
    pool.close().await;

    let guard = run_cli(&["guard"], &fresh, &[]);
    assert!(guard.status.success(), "guard: {}", guard.stderr);

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect after");
    let after_count: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("count after");
    let after_max: i64 =
        sqlx::query_scalar("SELECT coalesce(max(version), 0) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("max after");
    pool.close().await;

    assert_eq!(before_count, after_count, "guard must not add ledger rows");
    assert_eq!(before_max, after_max, "guard must not advance versions");

    drop_db(&db, &name).await;
}

/// LAW-B5: `edgequake migrate` must refuse (exit 78) when the ledger is ahead
/// of this binary — same STOP as serving boot (no "OK TO START THE SERVER").
#[tokio::test]
async fn cli_migrate_refuses_when_db_newer_than_binary() {
    let Some(db) = dev_db_url() else {
        eprintln!("SKIP: DATABASE_URL not set");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!("edgequake_cli_lawb5_{}_{}", std::process::id(), nanos);
    let Some(fresh) = create_fresh_db(&db, &name).await else {
        eprintln!("SKIP: could not create fresh DB");
        return;
    };

    // Fresh install applies the full train, then plant a fake applied version
    // beyond this binary's embedded max (foreign-branch ledger residue).
    let seed = run_cli(&[], &fresh, &[]);
    assert!(
        seed.status.success(),
        "fresh seed must succeed: stdout={} stderr={}",
        seed.stdout,
        seed.stderr
    );

    let pool = sqlx::PgPool::connect(&fresh).await.expect("connect");
    let embedded_max: i64 =
        sqlx::query_scalar("SELECT coalesce(max(version), 0) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("embedded max from seeded ledger");
    let newer = embedded_max + 1;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
         VALUES ($1, 'foreign-branch-seed', true, '\\x00', 1)",
    )
    .bind(newer)
    .execute(&pool)
    .await
    .expect("seed newer ledger row");
    pool.close().await;

    let out = run_cli(&[], &fresh, &[]);
    let code = out.status.code().unwrap_or(-1);
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert_eq!(
        code,
        edgequake_api::state::migration_bootstrap::BOOT_GATE_EXIT_CODE,
        "LAW-B5 migrate must exit 78: stdout={} stderr={}",
        out.stdout,
        out.stderr
    );
    assert!(
        combined.contains(edgequake_api::state::migration_bootstrap::BOOT_GATE_REFUSAL_PREFIX)
            || combined.contains("NEWER than this binary"),
        "must print downgrade refusal: {combined}"
    );
    assert!(
        !combined.contains("OK TO START THE SERVER"),
        "must not soft-exit OK TO START when DB is newer: {combined}"
    );

    drop_db(&db, &name).await;
}
