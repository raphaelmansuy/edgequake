//! Preflight checks for `edgequake migrate` / `edgequake migrate check`.
//!
//! Plain-English environment validation before any schema write. Failures are
//! actionable and never mutate the database.

#![cfg(feature = "postgres")]

use anyhow::{bail, Context, Result};
use sqlx::PgPool;

use edgequake_api::state::migration_bootstrap::{self, release_for_schema};

/// One preflight finding.
#[derive(Debug, Clone)]
pub struct PreflightCheck {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
    pub fix: String,
}

/// Full preflight report.
#[derive(Debug, Clone)]
pub struct PreflightReport {
    pub checks: Vec<PreflightCheck>,
    pub db_schema: Option<i64>,
    pub binary_schema: i64,
}

impl PreflightReport {
    pub fn all_ok(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }

    pub fn hard_failures(&self) -> impl Iterator<Item = &PreflightCheck> {
        self.checks.iter().filter(|c| !c.ok)
    }
}

/// Run all preflight probes (read-only).
pub async fn run_preflight(pool: &PgPool) -> Result<PreflightReport> {
    let mut checks = Vec::new();

    let (pg_major, pg_version) = probe_pg_version(pool).await?;
    let pg_ok = (16..=18).contains(&pg_major);
    checks.push(PreflightCheck {
        name: "postgresql_major",
        ok: pg_ok,
        detail: format!("PostgreSQL {pg_version} (major {pg_major})"),
        fix: "EdgeQuake supports PostgreSQL 16, 17, and 18 with pgvector. \
              Apache AGE is optional from v0.35.0. Use the GHCR edgequake-postgres \
              image, or any managed Postgres that ships pgvector (RDS / Cloud SQL)."
            .into(),
    });

    // Fresh databases have vector available but not yet CREATE EXTENSION'd
    // (migration 001 installs them). Fail only when the server cannot install.
    // Apache AGE is optional: plain eq_graph_* tables serve GraphRAG without it.
    let vector = probe_extension_status(pool, "vector").await?;
    checks.push(extension_check(
        "extension_vector",
        "pgvector",
        "vector",
        vector,
    ));

    let age = probe_extension_status(pool, "age").await?;
    checks.push(age_extension_check(age));

    let can_ddl = probe_can_create_temp(pool).await?;
    checks.push(PreflightCheck {
        name: "role_can_ddl",
        ok: can_ddl,
        detail: if can_ddl {
            "connected role can run DDL (temp table probe ok)".into()
        } else {
            "connected role cannot create objects".into()
        },
        fix: "Connect with a role that owns the EdgeQuake schema (typically the \
              `edgequake` superuser / owner used by compose)."
            .into(),
    });

    let dirty = probe_dirty_migrations(pool).await?;
    checks.push(PreflightCheck {
        name: "ledger_dirty",
        ok: dirty.is_empty(),
        detail: if dirty.is_empty() {
            "no dirty _sqlx_migrations rows".into()
        } else {
            format!(
                "dirty version(s): {} (success = false)",
                dirty
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        fix: "Inspect public._sqlx_migrations WHERE success = false; fix partial DDL, \
              delete the dirty row only after confirmation, then re-run migrate. \
              See docs/operations/upgrading.md § Recovery."
            .into(),
    });

    let db_schema = probe_max_applied(pool).await?;
    let binary_schema = migration_bootstrap::list_pending_migrations(pool)
        .await
        .ok()
        .map(|_| {
            // Prefer embedded max from pending+applied via description helper path.
            edgequake_migrate_manifest::load().compat_serve_max
        })
        .unwrap_or_else(|| edgequake_migrate_manifest::load().compat_serve_max);

    let binary_older = matches!(db_schema, Some(db) if db > binary_schema);
    checks.push(PreflightCheck {
        name: "binary_vs_database",
        ok: !binary_older,
        detail: if binary_older {
            format!(
                "PREFLIGHT_BINARY_OLDER: database schema {} (≈ {}) > binary schema {} (≈ {})",
                db_schema.unwrap_or(0),
                release_for_schema(db_schema.unwrap_or(0)),
                binary_schema,
                release_for_schema(binary_schema)
            )
        } else {
            format!(
                "database schema {} (≈ {}) ≤ binary schema {} (≈ {})",
                db_schema.unwrap_or(0),
                release_for_schema(db_schema.unwrap_or(0)),
                binary_schema,
                release_for_schema(binary_schema)
            )
        },
        fix: "Upgrade the migrator binary/image to ≥ the API version before migrate. \
              Never point an older binary at a newer ledger."
            .into(),
    });

    // Disk: best-effort via pg_tablespace_size / data directory probe.
    match probe_disk_headroom(pool).await {
        Ok((ok, detail)) => checks.push(PreflightCheck {
            name: "disk_headroom",
            ok,
            detail: if ok {
                detail
            } else {
                format!("PREFLIGHT_DISK: {detail}")
            },
            fix: "Free space on the PostgreSQL data volume. Large AGE rewrites and \
                  HNSW builds need substantial headroom."
                .into(),
        }),
        Err(e) => checks.push(PreflightCheck {
            name: "disk_headroom",
            ok: true, // soft — do not block when probe unavailable
            detail: format!("disk probe skipped ({e})"),
            fix: String::new(),
        }),
    }

    Ok(PreflightReport {
        checks,
        db_schema,
        binary_schema,
    })
}

/// Print the report; return Err when any hard check failed.
pub fn print_preflight_report(report: &PreflightReport) -> Result<()> {
    println!();
    println!(" PREFLIGHT");
    for c in &report.checks {
        let mark = if c.ok { "OK  " } else { "FAIL" };
        println!("  [{mark}] {}: {}", c.name, c.detail);
        if !c.ok && !c.fix.is_empty() {
            println!("         fix: {}", c.fix);
        }
    }
    println!();
    if report.all_ok() {
        println!("preflight: all checks passed");
        Ok(())
    } else {
        let fails: Vec<&str> = report.hard_failures().map(|c| c.name).collect();
        bail!("preflight failed: {}", fails.join(", "))
    }
}

/// `edgequake migrate check` — preflight + pending preview, no writes.
pub async fn run_migrate_check_cli() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL required for `edgequake migrate check`")?;
    let redacted = crate::redact_database_url(&database_url);
    println!("EdgeQuake migrate check v{}", env!("CARGO_PKG_VERSION"));
    println!("database: {redacted}");
    println!("MODE: CHECK-ONLY (no changes will be applied)");

    let bundle = edgequake_storage::PgPoolBundle::connect(&database_url)
        .await
        .context("PgPoolBundle connect failed")?;
    let report = run_preflight(&bundle.admin).await?;
    print_preflight_report(&report)?;

    let pending = migration_bootstrap::list_pending_migrations(&bundle.admin)
        .await
        .context("list pending migrations failed")?;
    crate::migrate_console::print_preflight(&pending);
    let irreversibles: Vec<i64> = pending
        .iter()
        .map(|(v, _)| *v)
        .filter(|v| crate::migrate_console::is_irreversible_drop(*v))
        .collect();
    migration_bootstrap::print_upgrade_path(
        report.db_schema,
        report.binary_schema,
        env!("CARGO_PKG_VERSION"),
        pending.len(),
        &irreversibles,
    );
    migration_bootstrap::print_cutover_duration_note(&bundle.admin).await;
    println!("next: edgequake migrate dry-run   # posture / drop readiness");
    println!("      edgequake migrate           # apply SAFE SCHEMA");
    Ok(())
}

async fn probe_pg_version(pool: &PgPool) -> Result<(i32, String)> {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(pool)
        .await
        .context("SHOW server_version")?;
    let major = version
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    Ok((major, version))
}

/// Installed vs installable-but-not-yet vs missing from the server image.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExtensionStatus {
    Installed(String),
    Available(String),
    Missing,
}

fn extension_check(
    name: &'static str,
    label: &str,
    ext: &str,
    status: ExtensionStatus,
) -> PreflightCheck {
    match status {
        ExtensionStatus::Installed(v) => PreflightCheck {
            name,
            ok: true,
            detail: format!("{label} {v}"),
            fix: String::new(),
        },
        ExtensionStatus::Available(v) => PreflightCheck {
            name,
            ok: true,
            detail: format!(
                "{label} {v} available (not yet CREATE EXTENSION; migrate will install)"
            ),
            fix: String::new(),
        },
        ExtensionStatus::Missing => PreflightCheck {
            name,
            ok: false,
            detail: format!("PREFLIGHT_EXTENSION: {ext} is not available on this server"),
            fix: format!(
                "Install {ext} into the PostgreSQL image (CREATE EXTENSION cannot succeed). \
                 Use edgequake/docker images / extension-pins.sh for PG 16–18."
            ),
        },
    }
}

/// Apache AGE is optional from v0.35.0. Missing is a note, not a hard fail.
fn age_extension_check(status: ExtensionStatus) -> PreflightCheck {
    match status {
        ExtensionStatus::Installed(v) => PreflightCheck {
            name: "extension_age",
            ok: true,
            detail: format!("Apache AGE {v}"),
            fix: String::new(),
        },
        ExtensionStatus::Available(v) => PreflightCheck {
            name: "extension_age",
            ok: true,
            detail: format!(
                "Apache AGE {v} available (optional; migrate installs only if present)"
            ),
            fix: String::new(),
        },
        ExtensionStatus::Missing => PreflightCheck {
            name: "extension_age",
            ok: true,
            detail: "Apache AGE not on this server — plain eq_graph_* tables will serve GraphRAG \
                     (RDS / Cloud SQL class). Drop is never required."
                .into(),
            fix: String::new(),
        },
    }
}

async fn probe_extension_status(pool: &PgPool, name: &str) -> Result<ExtensionStatus> {
    let installed: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = $1")
            .bind(name)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("probe installed extension {name}"))?;
    if let Some(v) = installed {
        return Ok(ExtensionStatus::Installed(v));
    }
    let available: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(default_version, '') FROM pg_available_extensions WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("probe available extension {name}"))?;
    match available {
        Some(v) => Ok(ExtensionStatus::Available(if v.is_empty() {
            "present".into()
        } else {
            v
        })),
        None => Ok(ExtensionStatus::Missing),
    }
}

async fn probe_can_create_temp(pool: &PgPool) -> Result<bool> {
    let res = sqlx::query("CREATE TEMP TABLE IF NOT EXISTS _eq_migrate_preflight (i int)")
        .execute(pool)
        .await;
    Ok(res.is_ok())
}

async fn probe_dirty_migrations(pool: &PgPool) -> Result<Vec<i64>> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Ok(vec![]);
    }
    let rows: Vec<i64> = sqlx::query_scalar(
        "SELECT version FROM public._sqlx_migrations WHERE NOT success ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    Ok(rows)
}

async fn probe_max_applied(pool: &PgPool) -> Result<Option<i64>> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Ok(None);
    }
    let max: Option<i64> =
        sqlx::query_scalar("SELECT max(version) FROM public._sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await
            .unwrap_or(None);
    Ok(max)
}

async fn probe_disk_headroom(pool: &PgPool) -> Result<(bool, String)> {
    // pg_database_size is always available; free-space requires superuser /
    // pg_stat_file which may be denied — treat as soft advisory.
    let size: i64 = sqlx::query_scalar("SELECT pg_database_size(current_database())")
        .fetch_one(pool)
        .await
        .context("pg_database_size")?;
    let mb = size as f64 / (1024.0 * 1024.0);
    // We cannot reliably read free bytes without host access; report size only.
    Ok((
        true,
        format!("database size ≈ {mb:.1} MiB (ensure free space ≥ 2× for heavy AGE/index steps)"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_all_ok_when_empty_failures() {
        let r = PreflightReport {
            checks: vec![PreflightCheck {
                name: "x",
                ok: true,
                detail: "ok".into(),
                fix: String::new(),
            }],
            db_schema: Some(168),
            binary_schema: 168,
        };
        assert!(r.all_ok());
    }

    #[test]
    fn available_extension_is_ok_for_fresh_database() {
        let c = extension_check(
            "extension_vector",
            "pgvector",
            "vector",
            ExtensionStatus::Available("0.8.0".into()),
        );
        assert!(c.ok);
        assert!(c.detail.contains("available"));
    }

    #[test]
    fn missing_vector_is_hard_fail() {
        let c = extension_check(
            "extension_vector",
            "pgvector",
            "vector",
            ExtensionStatus::Missing,
        );
        assert!(!c.ok);
        assert!(c.detail.contains("PREFLIGHT_EXTENSION"));
    }

    #[test]
    fn missing_age_is_optional_ok() {
        let c = age_extension_check(ExtensionStatus::Missing);
        assert!(c.ok);
        assert!(c.detail.contains("plain eq_graph_"));
    }
}
