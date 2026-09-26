//! SPEC-091 / SPEC-150 — `edgequake migrate` apply + drain verbs.
//!
//! Extracted from `main.rs` so the binary entry stays small (SRP).

use anyhow::{Context, Result};
use tracing::info;

use crate::migrate_console;
use crate::redact_database_url;

/// SPEC-090 F-090-20b: apply sqlx migrations + support reconcile on an admin pool.
///
/// SPEC-091 C3 (doc 15 §7, LAW-C5): migration 125 (the IRREVERSIBLE KV drop) is
/// never applied silently. On a pre-drop database where 125 is still pending,
/// this refuses unless the operator passes `--confirm-drop` (or sets
/// `EDGEQUAKE_MIGRATION_CONFIRM_DROP=1`). Databases where 125 already applied
/// (e.g. dev) are unaffected. One irreversible op per release (LD-07).
#[cfg(feature = "postgres")]
pub(crate) async fn run_migrate_cli(args: &[String]) -> Result<()> {
    // SAFETY: process-local flag for migrate CLI path; set before any bootstrap work.
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL required for `edgequake migrate`")?;
    let redacted = redact_database_url(&database_url);
    migrate_console::print_banner(env!("CARGO_PKG_VERSION"), &redacted);
    migrate_console::print_first_principles();
    info!(database = %redacted, "edgequake migrate: connecting admin pool");

    let bundle = match edgequake_storage::PgPoolBundle::connect(&database_url).await {
        Ok(b) => b,
        Err(e) => {
            migrate_console::print_failure_hint(&e);
            return Err(e).context("PgPoolBundle connect failed");
        }
    };

    let pending =
        match edgequake_api::state::migration_bootstrap::list_pending_migrations(&bundle.admin)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                migrate_console::print_failure_hint(&e);
                return Err(e).context("list pending migrations failed");
            }
        };
    migrate_console::print_preflight(&pending);
    let confirmed = drop_confirmed(args);

    // SPEC-091 Doc 17 (LAW-C5 scope): consent for irreversible drops is
    // required only when there is something to lose. On a FRESH install (zero
    // applied migrations) no legacy data exists, so the drop guards are
    // trivially green and `--confirm-drop` is not required — first-boot UX
    // (`make dev` cold start, fresh installs) stays one visible step. Any
    // database with applied migrations keeps the explicit gate.
    let fresh_install = pending.is_empty()
        || edgequake_api::state::migration_bootstrap::is_fresh_database(&bundle.admin)
            .await
            .unwrap_or(false);
    let drop_gate_open = confirmed || fresh_install;
    if fresh_install && !confirmed {
        let irreversibles: Vec<i64> = pending
            .iter()
            .map(|(v, _)| *v)
            .filter(|v| {
                *v == migrate_console::KV_DROP_MIGRATION
                    || *v == migrate_console::VECTOR_DROP_MIGRATION
                    || *v == migrate_console::FLEET_VECTOR_DROP_MIGRATION
            })
            .collect();
        if !irreversibles.is_empty() {
            println!(
                "fresh install (no applied migrations): irreversible migration(s) {irreversibles:?} \
                 cannot destroy data — nothing legacy exists; proceeding without --confirm-drop."
            );
        }
    }
    migrate_console::print_apply_intent(&pending, drop_gate_open);

    // First principles (LD-07 / LAW-C5): consent gates *destroy-data* steps only.
    // When an irreversible drop is pending without confirm, still apply ALL
    // expandable SAFE SCHEMA migrations (including those that sit *after* the
    // gated drop — e.g. 132 behind 131), then soft-exit 0 so `make_dev` can boot.
    if !drop_gate_open
        && pending
            .iter()
            .any(|(v, _)| migrate_console::is_irreversible_drop(*v))
    {
        let expandables = migrate_console::pending_expandable_versions(&pending);
        let defer_142 = edgequake_storage::any_legacy_rows(&bundle.query)
            .await
            .context("legacy census before expandable migrate")?;
        let expandables_to_apply: Vec<i64> = expandables
            .iter()
            .copied()
            .filter(|v| {
                !(defer_142
                    && edgequake_api::state::migration_bootstrap::is_legacy_cutover_assert(*v))
            })
            .collect();
        if !expandables_to_apply.is_empty() {
            println!(
                "applying SAFE SCHEMA (expandable) migration(s) {expandables_to_apply:?} \
                 (DROP OLD deferred until --confirm-drop)…"
            );
            if defer_142
                && expandables.iter().any(|v| {
                    edgequake_api::state::migration_bootstrap::is_legacy_cutover_assert(*v)
                })
            {
                println!(
                    "  note: SPEC-105 migration 142 deferred — durable legacy rows remain; \
                     finish --confirm-drop (125/126/131) first."
                );
            }
            let report =
                match edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(
                    &bundle.admin,
                )
                .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        migrate_console::print_failure_hint(&e);
                        return Err(e).context("partial migrate (expandables) failed");
                    }
                };
            let applied: Vec<(i64, String)> = report
                .applied_versions
                .iter()
                .copied()
                .map(|v| {
                    (
                        v,
                        edgequake_api::state::migration_bootstrap::migration_description(v),
                    )
                })
                .collect();
            migrate_console::print_applied_this_run(&applied);
            migrate_console::print_post_hooks(&bundle.admin).await;
        } else if defer_142 {
            println!(
                "no SAFE SCHEMA to apply this run (SPEC-105 migration 142 deferred while \
                 durable legacy rows remain)."
            );
        }

        let remaining =
            edgequake_api::state::migration_bootstrap::list_pending_migrations(&bundle.admin)
                .await
                .context("re-list pending after expandable apply")?;
        let remaining_versions: Vec<i64> = remaining.iter().map(|(v, _)| *v).collect();
        if remaining.is_empty() {
            return Ok(());
        }
        let defer_142_after = edgequake_storage::any_legacy_rows(&bundle.query)
            .await
            .context("legacy census after expandable migrate")?;
        if edgequake_api::state::migration_bootstrap::pending_ok_to_serve(
            &remaining_versions,
            defer_142_after,
        ) {
            match edgequake_storage::migration_engine::advisor::posture(&bundle.query).await {
                Ok(p) => migrate_console::print_guard(&p, &p.residue),
                Err(e) => eprintln!("  (readiness guard unavailable: {e})"),
            }
            migrate_console::print_irreversible_pending_soft_exit(&remaining);
            info!(
                remaining = ?remaining_versions,
                defer_legacy_cutover_assert = defer_142_after,
                "edgequake migrate: expandable train done; irreversible drop(s)/deferred 142 left"
            );
            return Ok(());
        }

        // Irreversible is next and expandable work sits behind it — classic refuse.
        let blocking = remaining_versions
            .iter()
            .copied()
            .find(|v| migrate_console::is_irreversible_drop(*v))
            .unwrap_or(remaining_versions[0]);
        match edgequake_storage::migration_engine::advisor::posture(&bundle.query).await {
            Ok(p) => migrate_console::print_guard(&p, &p.residue),
            Err(e) => eprintln!("  (readiness guard unavailable: {e})"),
        }
        migrate_console::print_blocked_by_irreversible(blocking);
        anyhow::bail!("migration {blocking} requires explicit --confirm-drop");
    }

    let pending_count = pending.len();
    if pending_count > 0 {
        println!("applying {pending_count} migration(s) + support reconcile on admin pool…");
    } else {
        println!("applying migrations + support reconcile on admin pool…");
    }

    let report =
        match edgequake_api::state::migration_bootstrap::run_postgres_migrations(&bundle.admin)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                migrate_console::print_failure_hint(&e);
                let msg = e.to_string();
                if msg.contains("MIGRATE_LOCK_BUSY") {
                    eprintln!("{msg}");
                    std::process::exit(
                        edgequake_api::state::migration_bootstrap::runner::MIGRATE_LOCK_EXIT_CODE,
                    );
                }
                return Err(e).context("migrate failed");
            }
        };

    let applied: Vec<(i64, String)> = report
        .applied_versions
        .iter()
        .copied()
        .map(|v| {
            (
                v,
                edgequake_api::state::migration_bootstrap::migration_description(v),
            )
        })
        .collect();
    migrate_console::print_applied_this_run(&applied);
    if applied
        .iter()
        .any(|(v, _)| *v == migrate_console::KV_DROP_MIGRATION)
    {
        migrate_console::print_kv_drop_applied();
    }
    migrate_console::print_post_hooks(&bundle.admin).await;

    info!(
        pending_before = report.pending_before,
        latest = ?report.latest_version,
        applied = report.applied_versions.len(),
        "edgequake migrate complete"
    );
    migrate_console::print_summary(
        report.pending_before,
        report.latest_version,
        report.applied_versions.len(),
    );
    Ok(())
}

/// SPEC-091 C3: was the irreversible drop explicitly confirmed? Via the
/// `--confirm-drop` flag or the `EDGEQUAKE_MIGRATION_CONFIRM_DROP` env var.
#[cfg(feature = "postgres")]
pub(crate) fn drop_confirmed(args: &[String]) -> bool {
    migrate_console::drop_consent_from_args(args)
        || matches!(
            std::env::var("EDGEQUAKE_MIGRATION_CONFIRM_DROP")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "on" | "yes"
        )
}

/// SPEC-150: run the migration engine in the foreground until jobs complete
/// (or `--timeout` seconds elapse). Does not apply sqlx schema — call
/// `edgequake migrate` first.
pub(crate) async fn run_migrate_drain(args: &[String]) -> Result<()> {
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    if std::env::var("EDGEQUAKE_MIGRATION_MODE").is_err() {
        std::env::set_var("EDGEQUAKE_MIGRATION_MODE", "automatic");
    }
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL required for `edgequake migrate drain`")?;
    let redacted = redact_database_url(&database_url);
    println!("EdgeQuake migrate drain v{}", env!("CARGO_PKG_VERSION"));
    println!("database: {redacted}");

    let timeout_secs = args
        .windows(2)
        .find(|w| w[0] == "--timeout")
        .and_then(|w| w[1].parse::<u64>().ok())
        .or_else(|| {
            std::env::var("EDGEQUAKE_MIGRATE_DRAIN_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(3600);

    let bundle = edgequake_storage::PgPoolBundle::connect(&database_url)
        .await
        .context("PgPoolBundle connect failed")?;

    let prefix = std::env::var("EDGEQUAKE_TABLE_PREFIX").unwrap_or_else(|_| "edgequake".into());
    let kv_table = edgequake_storage::adapters::postgres::qualified_kv_table_name(&prefix);
    let vectors_table = format!("public.eq_{prefix}_vectors");

    println!("draining migration engine jobs (timeout={timeout_secs}s, mode=automatic)…");
    let run =
        edgequake_storage::run_drain_foreground(bundle.admin.clone(), kv_table, vectors_table);
    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), run).await {
        Ok(Ok(())) => {
            println!("migrate drain complete");
            Ok(())
        }
        Ok(Err(e)) => Err(e).context("migrate drain failed"),
        Err(_) => anyhow::bail!(
            "migrate drain timed out after {timeout_secs}s — re-run `edgequake migrate drain`"
        ),
    }
}
