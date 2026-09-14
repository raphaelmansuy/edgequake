//! SPEC-091 Migration Console — CLI verbs (doc 15 §7).
//!
//! Thin renderers / controllers over `edgequake_storage::migration_engine::advisor`.
//! The advisor derives the posture from the live schema (SSOT); this module only
//! connects, calls the advisor, prints, and — for the gated write verbs — applies
//! the guardrail before mutating. Read-only verbs (console/plan/guard/family list)
//! are always available; write verbs are flag-gated behind
//! `EDGEQUAKE_MIGRATION_CONSOLE` (LD-07).

use anyhow::{bail, Context, Result};
use sqlx::PgPool;

use edgequake_storage::migration_engine::advisor::{self, FamilyMode};
use edgequake_storage::migration_engine::lease::{self, JobControl};
use edgequake_storage::PgPoolBundle;

use crate::migrate_console;

/// LD-07: write verbs (family set, pause/resume/cancel) are gated behind this
/// flag; read-only verbs are always available.
const WRITE_GATE_ENV: &str = "EDGEQUAKE_MIGRATION_CONSOLE";

fn writes_enabled() -> bool {
    matches!(
        std::env::var(WRITE_GATE_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "on" | "1" | "true"
    )
}

fn redacted() -> String {
    std::env::var("DATABASE_URL")
        .map(|u| crate::redact_database_url(&u))
        .unwrap_or_else(|_| "(DATABASE_URL unset)".to_string())
}

async fn connect() -> Result<PgPoolBundle> {
    let url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL required for `edgequake migrate <verb>`")?;
    PgPoolBundle::connect(&url)
        .await
        .context("PgPoolBundle connect failed")
}

/// `edgequake migrate console [--watch]` — the intelligent dashboard.
pub async fn run_console(watch: bool) -> Result<()> {
    let bundle = connect().await?;
    loop {
        let posture = advisor::posture(&bundle.query).await?;
        let guidance = advisor::derive_guidance(&posture);
        let actions = advisor::derive_actions(&posture);
        migrate_console::print_console_banner(env!("CARGO_PKG_VERSION"), &redacted(), &posture);
        migrate_console::print_family_table(&posture);
        migrate_console::print_instructions(&guidance);
        migrate_console::print_actions(&actions);
        if !watch {
            break;
        }
        println!("\n --watch: refreshing every 5s (Ctrl-C to exit)");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        println!("\n──────────────────────────────────────────────────────────────────────────\n");
    }
    Ok(())
}

/// `edgequake migrate plan` — the ordered, live-derived runbook only.
pub async fn run_plan() -> Result<()> {
    let bundle = connect().await?;
    let posture = advisor::posture(&bundle.query).await?;
    let guidance = advisor::derive_guidance(&posture);
    println!("EdgeQuake migrate plan v{}", env!("CARGO_PKG_VERSION"));
    println!("database: {}", redacted());
    println!(
        "{}",
        migrate_console::copy_engine_image_skew_hint(env!("CARGO_PKG_VERSION"))
    );
    migrate_console::print_instructions(&guidance);
    Ok(())
}

/// `edgequake migrate guard [--family <name>]` — read-only readiness probe.
pub async fn run_guard(family: Option<String>) -> Result<()> {
    let bundle = connect().await?;
    let posture = advisor::posture(&bundle.query).await?;
    println!("EdgeQuake migrate guard v{}", env!("CARGO_PKG_VERSION"));
    println!("database: {}", redacted());
    println!(
        "{}",
        migrate_console::copy_engine_image_skew_hint(env!("CARGO_PKG_VERSION"))
    );
    match family {
        None => migrate_console::print_guard(&posture, &posture.residue),
        Some(name) => print_family_guard(&posture, &name),
    }
    Ok(())
}

/// Per-family readiness detail (used by `guard --family`).
fn print_family_guard(posture: &advisor::MigrationPosture, name: &str) {
    let upper = name.to_ascii_uppercase();
    let Some(f) = posture.family(&upper) else {
        eprintln!("unknown family '{name}' (known: {})", known_families());
        return;
    };
    println!();
    println!(" GUARD family {}", f.family);
    println!("  mode:        {}", f.mode.as_str());
    println!("  phase:       {}", f.phase.as_str());
    println!("  durable:     {}", f.durable);
    println!("  residue:     {} row(s) not yet typed", f.kv_residue_rows);
    println!(
        "  typed rows:  {} ({})",
        f.typed_rows,
        f.typed_tables.join(", ")
    );
    if let Some(v) = &f.verify {
        println!(
            "  verify:      {} (expected={} actual={} sampled={} mismatches={})",
            if v.passes() { "PASS" } else { "FAIL" },
            v.expected,
            v.actual,
            v.sampled,
            v.mismatches
        );
    }
    if let Some(j) = &f.backfill {
        println!(
            "  backfill:    {} {} ({} done / {:?} total)",
            j.step_id, j.state, j.processed_count, j.estimated_total
        );
    }
}

fn known_families() -> String {
    advisor::FAMILIES
        .iter()
        .map(|s| s.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// `edgequake migrate family list` — per-family posture table.
pub async fn run_family_list() -> Result<()> {
    let bundle = connect().await?;
    let posture = advisor::posture(&bundle.query).await?;
    println!(
        "EdgeQuake migrate family list v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("database: {}", redacted());
    migrate_console::print_family_table(&posture);
    Ok(())
}

/// `edgequake migrate family set <family> <mode> [--yes]` — gated flag change.
///
/// Flags are env-owned, so a "set" cannot mutate the running server's
/// environment; it (a) validates the transition against the LIVE posture
/// (never a stale one), (b) refuses with the gate reason when unsafe, and
/// (c) prints the exact export to apply + restart.
pub async fn run_family_set(family: &str, mode: &str, yes: bool) -> Result<()> {
    if !writes_enabled() {
        bail!("✗ `family set` is gated (LD-07): set {WRITE_GATE_ENV}=on to enable write verbs.");
    }
    let upper = family.to_ascii_uppercase();
    let spec = advisor::FAMILIES
        .iter()
        .find(|s| s.name == upper)
        .with_context(|| format!("unknown family '{family}' (known: {})", known_families()))?;

    let target = parse_mode(spec.is_chunk, mode)?;

    // Re-derive the posture LIVE — the guardrail must reflect the schema now.
    let bundle = connect().await?;
    let posture = advisor::posture(&bundle.query).await?;
    let fp = posture
        .family(spec.name)
        .with_context(|| format!("no posture for family {}", spec.name))?;

    // Idempotent: already on the requested mode → no-op success.
    if fp.mode == target {
        println!("✓ {} already {} (no-op)", spec.name, target.as_str());
        return Ok(());
    }

    // Consult the derived action plane (DRY — the gate logic lives in the
    // advisor, not here). Rollback modes (kv/dual) share the `family.set kv`
    // gate; the forward flip uses `family.set relational`.
    let verb = match target {
        FamilyMode::Relational => "family.set relational",
        FamilyMode::Kv | FamilyMode::Dual => "family.set kv",
    };
    let actions = advisor::derive_actions(&posture);
    if let Some(a) = actions
        .iter()
        .find(|a| a.verb == verb && a.target == spec.name)
    {
        if !a.enabled {
            bail!(
                "✗ {}",
                a.gate_reason.clone().unwrap_or_else(|| format!(
                    "cannot set {} to {}",
                    spec.name,
                    target.as_str()
                ))
            );
        }
    }

    if !yes {
        println!(
            "About to set {}={} (currently {}). Re-run with --yes to apply.",
            spec.env_flag,
            target.as_str(),
            fp.mode.as_str()
        );
        return Ok(());
    }

    // Persist for this process and print the exact operator instruction.
    std::env::set_var(spec.env_flag, target.as_str());
    println!(
        "✓ {} set to {} for this process.",
        spec.env_flag,
        target.as_str()
    );
    println!();
    println!("  Apply it to the server (flags are env-owned):");
    println!("    $ export {}={}", spec.env_flag, target.as_str());
    println!("    $ make stop && make dev   # restart so the flag takes effect");
    Ok(())
}

fn parse_mode(is_chunk: bool, mode: &str) -> Result<FamilyMode> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "relational" => Ok(FamilyMode::Relational),
        "kv" => Ok(FamilyMode::Kv),
        "dual" if is_chunk => Ok(FamilyMode::Dual),
        other => bail!(
            "invalid mode '{other}' (valid: relational, kv{})",
            if is_chunk { ", dual" } else { "" }
        ),
    }
}

/// `edgequake migrate pause|resume|cancel <step_id>` — job control via the
/// ledger state machine, surfacing the explicit gate reason on refusal.
pub async fn run_job_control(verb: &str, step_id: &str) -> Result<()> {
    if !writes_enabled() {
        bail!("✗ `{verb}` is gated (LD-07): set {WRITE_GATE_ENV}=on to enable write verbs.");
    }
    let control = match verb {
        "pause" => JobControl::Pause,
        "resume" => JobControl::Resume,
        "cancel" => JobControl::Cancel,
        other => bail!("unknown job-control verb '{other}' (pause|resume|cancel)"),
    };
    let bundle = connect().await?;
    let job_id = find_job_id(&bundle.query, step_id).await?;
    match lease::control_job(&bundle.admin, job_id, control).await {
        Ok(new_state) => {
            println!("✓ {verb} {step_id}: state -> '{new_state}'");
            Ok(())
        }
        // control_job carries the explicit reason ("cannot X in state 'Y'");
        // surface it without the StorageError variant prefix.
        Err(e) => bail!("✗ {}", gate_message(&e)),
    }
}

/// Extract the operator-readable reason from a control_job error.
fn gate_message(e: &edgequake_storage::error::StorageError) -> String {
    use edgequake_storage::error::StorageError;
    match e {
        StorageError::InvalidQuery(m) => m.clone(),
        other => other.to_string(),
    }
}

/// Latest job id for a step id (the ledger may hold one row per generation).
async fn find_job_id(pool: &PgPool, step_id: &str) -> Result<sqlx::types::Uuid> {
    let id: Option<sqlx::types::Uuid> = sqlx::query_scalar(
        "SELECT job_id FROM edgequake.edgequake_migration_job \
         WHERE step_id = $1 ORDER BY schema_generation DESC LIMIT 1",
    )
    .bind(step_id)
    .fetch_optional(pool)
    .await
    .context("migration ledger read failed (apply migration 106?)")?;
    id.with_context(|| format!("no migration job found for step '{step_id}'"))
}
