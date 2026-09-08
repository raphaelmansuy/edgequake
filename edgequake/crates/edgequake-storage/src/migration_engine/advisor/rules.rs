//! SPEC-091 Migration Console — the pure rule engine (LAW-C2/C6).
//!
//! Every function here is a pure function over the derived posture: no I/O, no
//! env reads, no database. That makes the entire guidance surface unit-testable
//! with fixture postures (the `contract_spec091_advisor_phases` suite below) and
//! lets a future second renderer (WebUI/API) reuse it without re-derivation.

use super::types::{
    CutoverPhase, FamilyMode, FamilyPhase, FamilyPosture, GuardedAction, Guidance, InstrKind,
    Instruction, JobSnapshot, MigrationPosture, VerifySummary,
};
use crate::migration_engine::lease::JobControl;
use crate::migration_engine::MigrationMode;

/// Derive one family's lifecycle phase from its facts (doc 15 §4 state machine).
///
/// `global_ready_to_drop` is computed by the caller (it depends on all
/// families), so a relational family reports `ReadyToDrop` only when the whole
/// system is drained — otherwise `Flipped` (still soaking).
pub fn derive_family_phase(
    mode: FamilyMode,
    is_chunk: bool,
    backfill: Option<&JobSnapshot>,
    verify: Option<&VerifySummary>,
    residue_rows: i64,
    kv_dropped: bool,
    global_ready_to_drop: bool,
) -> FamilyPhase {
    if kv_dropped {
        return FamilyPhase::Dropped;
    }
    match mode {
        FamilyMode::Relational => {
            if global_ready_to_drop {
                FamilyPhase::ReadyToDrop
            } else {
                FamilyPhase::Flipped
            }
        }
        FamilyMode::Kv | FamilyMode::Dual => {
            if let Some(job) = backfill {
                if job.state == "verifying" {
                    return FamilyPhase::Verifying;
                }
                if job.is_active() {
                    return FamilyPhase::Backfilling;
                }
            }
            // No active job: readiness to flip. The chunk family additionally
            // requires the content verify to pass; other families treat the
            // 125-style typed check (residue == 0) as their verification.
            let verified = if is_chunk {
                verify.map(|v| v.passes()).unwrap_or(false)
            } else {
                true
            };
            if residue_rows == 0 && verified {
                FamilyPhase::ReadyToFlip
            } else {
                match mode {
                    FamilyMode::Kv => FamilyPhase::NotStarted,
                    FamilyMode::Dual => FamilyPhase::DualWriting,
                    FamilyMode::Relational => unreachable!("handled above"),
                }
            }
        }
    }
}

/// Aggregate cutover phase (doc 15 §4): the global "where am I".
pub fn derive_cutover_phase(posture: &MigrationPosture) -> CutoverPhase {
    if posture.kv_store_dropped {
        return CutoverPhase::Dropped;
    }
    if posture.global_ready_to_drop() {
        return CutoverPhase::ReadyToDrop;
    }
    let durable: Vec<&FamilyPosture> = posture.families.iter().filter(|f| f.durable).collect();
    if !durable.is_empty() && durable.iter().all(|f| f.phase == FamilyPhase::NotStarted) {
        return CutoverPhase::NotStarted;
    }
    CutoverPhase::InProgress
}

/// Map a derived posture to ordered instructions (LAW-C6: a sentence per metric).
pub fn derive_guidance(posture: &MigrationPosture) -> Guidance {
    let mut g = Guidance::default();
    let mut push = |kind: InstrKind, summary: String, command: Option<String>, evidence: String| {
        g.instructions.push(Instruction {
            ordinal: 0, // renumbered at the end
            kind,
            summary,
            command,
            gate: None,
            evidence,
        });
    };

    // EC-C3 — fail-closed when the engine ledger itself is missing.
    if !posture.engine_installed {
        push(
            InstrKind::Blocked,
            "Migration engine ledger (edgequake_migration_job) not found — apply migration 106 / \
             run `edgequake migrate` to install the engine before guided cutover."
                .to_string(),
            Some("edgequake migrate".to_string()),
            "engine not installed".to_string(),
        );
    }

    // EC-C1 — the stale-flag bug, now caught for every family still pointing at
    // a dropped store.
    for f in posture
        .families
        .iter()
        .filter(|f| posture.kv_store_dropped && f.mode.writes_kv())
    {
        push(
            InstrKind::Blocked,
            format!(
                "{}={} but the KV store is DROPPED — writes hit a missing relation (42P01). \
                 Set {}=relational and restart immediately.",
                f.env_flag,
                f.mode.as_str(),
                f.env_flag
            ),
            Some(format!("export {}=relational", f.env_flag)),
            "kv_store_dropped".to_string(),
        );
    }

    // LD-14 — stale chunk-vector backend after migration 126 fleet drop.
    if posture.vector.chunk_fleet_dropped && posture.vector.backend == "legacy_tables" {
        push(
            InstrKind::Blocked,
            format!(
                "{}=legacy_tables but legacy chunk-vector tables are DROPPED — reads miss \
                 chunk_embeddings. Set {}=chunk_embeddings and restart.",
                crate::vector_backend::VECTOR_BACKEND_ENV,
                crate::vector_backend::VECTOR_BACKEND_ENV,
            ),
            Some(format!(
                "export {}=chunk_embeddings",
                crate::vector_backend::VECTOR_BACKEND_ENV
            )),
            "vector_legacy_dropped".to_string(),
        );
    }

    // Post-drop terminal state.
    if posture.kv_store_dropped {
        if posture
            .families
            .iter()
            .all(|f| f.mode == FamilyMode::Relational)
        {
            push(
                InstrKind::Done,
                "Generic KV store retired. Chunk text authority = public.chunks; all families \
                 relational. Nothing further to do."
                    .to_string(),
                None,
                "kv_store_dropped; all relational".to_string(),
            );
        }
        return finalize(g);
    }

    // Engine mode gate (fact #8).
    if posture.engine_mode == MigrationMode::Off {
        push(
            InstrKind::Blocked,
            "Engine is `off` — jobs register but never run. Set EDGEQUAKE_MIGRATION_MODE=automatic \
             to execute, or `verify` to only observe."
                .to_string(),
            Some("export EDGEQUAKE_MIGRATION_MODE=automatic".to_string()),
            "engine_mode=off".to_string(),
        );
    }

    // Per-family phase guidance.
    for f in &posture.families {
        // A failed backfill always surfaces, regardless of phase.
        if let Some(job) = &f.backfill {
            if job.state == "failed" {
                push(
                    InstrKind::Blocked,
                    format!(
                        "{} backfill FAILED: {}. Investigate and re-run the job.",
                        f.family,
                        job.last_error.as_deref().unwrap_or("unknown error")
                    ),
                    None,
                    format!("step={} state=failed", job.step_id),
                );
            }
        }
        match f.phase {
            FamilyPhase::NotStarted if f.durable => {
                // Data still lives only in KV — point at the remedy (backfill,
                // then flip) so the runbook is never empty at the earliest phase.
                push(
                    InstrKind::Action,
                    format!(
                        "ACTION: {} has {} un-migrated row(s) still in KV. Run the family backfill \
                         / migration, then set {}=relational.",
                        f.family, f.kv_residue_rows, f.env_flag
                    ),
                    None,
                    format!("residue={}", f.kv_residue_rows),
                );
            }
            FamilyPhase::DualWriting => {
                push(
                    InstrKind::Action,
                    format!(
                        "ACTION: {} is dual-writing with {} row(s) not yet backfilled. Ensure the \
                         engine is draining them (EDGEQUAKE_MIGRATION_MODE=automatic) before flipping.",
                        f.family, f.kv_residue_rows
                    ),
                    Some("export EDGEQUAKE_MIGRATION_MODE=automatic".to_string()),
                    format!("residue={}", f.kv_residue_rows),
                );
            }
            FamilyPhase::Backfilling => {
                let pct = f
                    .backfill
                    .as_ref()
                    .and_then(|j| j.completion_pct)
                    .map(|p| format!("{p:.1}%"))
                    .unwrap_or_else(|| "?".to_string());
                let eta = f
                    .backfill
                    .as_ref()
                    .and_then(|j| j.eta_seconds)
                    .map(fmt_eta)
                    .unwrap_or_else(|| "unknown ETA".to_string());
                push(
                    InstrKind::Wait,
                    format!(
                        "WAIT: {} backfill {} ({}). Do not flip flags yet.",
                        f.family, pct, eta
                    ),
                    None,
                    format!(
                        "state={} residue={}",
                        f.backfill.as_ref().map(|j| j.state.as_str()).unwrap_or("?"),
                        f.kv_residue_rows
                    ),
                );
            }
            FamilyPhase::Verifying => {
                push(
                    InstrKind::Wait,
                    format!("WAIT: {} backfill is verifying sampled content.", f.family),
                    None,
                    format!(
                        "step={}",
                        f.backfill
                            .as_ref()
                            .map(|j| j.step_id.as_str())
                            .unwrap_or("?")
                    ),
                );
            }
            FamilyPhase::ReadyToFlip => {
                let evidence = if let Some(v) = f.verify {
                    format!(
                        "verified {} samples, {} mismatches; residue {}",
                        v.sampled, v.mismatches, f.kv_residue_rows
                    )
                } else {
                    format!("residue {} (typed-verified)", f.kv_residue_rows)
                };
                push(
                    InstrKind::Action,
                    format!(
                        "ACTION: set {}=relational and restart. {}.",
                        f.env_flag, evidence
                    ),
                    Some(format!("export {}=relational", f.env_flag)),
                    evidence,
                );
            }
            _ => {}
        }
    }

    // Drop path.
    let all_relational = posture
        .families
        .iter()
        .all(|f| f.mode == FamilyMode::Relational);
    if all_relational {
        if posture.residue.total() == 0 {
            push(
                InstrKind::Confirm,
                "CONFIRM: all families relational and KV drained (0 durable rows outside typed \
                 tables). Apply migration 125 (IRREVERSIBLE): run `edgequake migrate` with \
                 --confirm-drop. Ensure the LD-07 soak window has elapsed first; rollback after \
                 the drop = restore from backup."
                    .to_string(),
                Some("edgequake migrate --confirm-drop".to_string()),
                "residue=0; all relational".to_string(),
            );
        } else {
            push(
                InstrKind::Blocked,
                format!(
                    "BLOCKED: {} durable KV rows not yet in typed tables ({}). Run the family \
                     backfills (117-122 / engine) and re-check.",
                    posture.residue.total(),
                    posture.residue.breakdown()
                ),
                None,
                format!("residue={}", posture.residue.total()),
            );
        }
    }

    finalize(g)
}

fn finalize(mut g: Guidance) -> Guidance {
    for (i, ins) in g.instructions.iter_mut().enumerate() {
        ins.ordinal = (i + 1) as u32;
    }
    g
}

/// Derive the gated action plane for a posture (doc 15 §6 guardrail matrix).
/// Every `enabled` flag is computed from the posture; the CLI re-checks live at
/// execution time before mutating.
pub fn derive_actions(posture: &MigrationPosture) -> Vec<GuardedAction> {
    let mut actions = Vec::new();

    // Job-control verbs for each family with an engine job.
    for f in &posture.families {
        let Some(job) = &f.backfill else { continue };
        if job.job_id.is_none() {
            continue;
        }
        for (verb, control) in [
            ("pause", JobControl::Pause),
            ("resume", JobControl::Resume),
            ("cancel", JobControl::Cancel),
        ] {
            match control.transition(&job.state) {
                Some(_) => actions.push(GuardedAction::enabled(verb, &job.step_id)),
                None => actions.push(GuardedAction::gated(
                    verb,
                    &job.step_id,
                    format!("cannot {verb}: job is '{}'", job.state),
                )),
            }
        }
    }

    // Family cutover verbs.
    for f in &posture.families {
        // set → relational. Offered whenever the family is not already relational.
        // Post-drop this is the EC-C1 *remedy* (a stale dual/kv flag must be
        // flipped to relational), so it is enabled even though phase == Dropped.
        if f.mode != FamilyMode::Relational {
            if f.phase == FamilyPhase::ReadyToFlip || posture.kv_store_dropped {
                actions.push(GuardedAction::enabled("family.set relational", f.family));
            } else {
                let reason = match f.phase {
                    FamilyPhase::Backfilling | FamilyPhase::DualWriting => {
                        format!("cannot flip {}: backfill incomplete", f.family)
                    }
                    FamilyPhase::Verifying => {
                        format!("cannot flip {}: verify in progress", f.family)
                    }
                    FamilyPhase::NotStarted => {
                        format!("cannot flip {}: no typed migration yet", f.family)
                    }
                    _ if f.kv_residue_rows > 0 => {
                        format!(
                            "cannot flip {}: {} un-migrated rows",
                            f.family, f.kv_residue_rows
                        )
                    }
                    _ => format!("cannot flip {}: not ready", f.family),
                };
                actions.push(GuardedAction::gated(
                    "family.set relational",
                    f.family,
                    reason,
                ));
            }
        }

        // set → kv / dual (rollback). Only meaningful when currently relational.
        // Refused once the store is dropped (EC-C1: a rollback would 42P01).
        if f.mode == FamilyMode::Relational {
            if posture.kv_store_dropped {
                actions.push(GuardedAction::gated(
                    "family.set kv",
                    f.family,
                    format!(
                        "cannot roll back {} to kv: KV store dropped — would 42P01 (the bug)",
                        f.family
                    ),
                ));
            } else {
                actions.push(GuardedAction::enabled("family.set kv", f.family));
            }
        }
    }

    // The drop is never executed by the CLI — reported as a gated action only.
    if !posture.kv_store_dropped {
        let mut drop_action = if posture.global_ready_to_drop() {
            GuardedAction::enabled("drop", "kv-store")
        } else {
            GuardedAction::gated(
                "drop",
                "kv-store",
                format!(
                    "{} durable rows + {} families not relational",
                    posture.residue.total(),
                    posture
                        .families
                        .iter()
                        .filter(|f| f.mode != FamilyMode::Relational)
                        .count()
                ),
            )
        };
        drop_action.requires_confirmation = true;
        drop_action.irreversible = true;
        actions.push(drop_action);
    }

    // SPEC-091 W4 — legacy chunk-vector fleet retirement. Never executed by the
    // CLI (reported as a gated action only); the physical DROP runs via
    // `edgequake migrate --confirm-drop` → migration 126's in-SQL guard.
    if !posture.vector.dropped {
        let v = &posture.vector;
        let mut drop_action = if v.retirable() {
            GuardedAction::enabled("drop", "vector-legacy")
        } else {
            let reason = if !matches!(v.backend.as_str(), "typed_embeddings" | "chunk_embeddings") {
                format!(
                    "cannot drop vector-legacy: backend is '{}' (flip to typed_embeddings first)",
                    v.backend
                )
            } else if v.uncovered_chunk_rows > 0 {
                format!(
                    "cannot drop vector-legacy: {} legacy chunk rows uncovered in typed SSOT \
                     (missing_spine={}, missing_embedding={})",
                    v.uncovered_chunk_rows,
                    v.uncovered_chunk_missing_spine_rows,
                    v.uncovered_chunk_missing_embedding_rows
                )
            } else if !v.verify_chunk.map(|x| x.passes()).unwrap_or(false) {
                "cannot drop vector-legacy: chunk verify not passing".to_string()
            } else {
                "cannot drop vector-legacy: not ready".to_string()
            };
            GuardedAction::gated("drop", "vector-legacy", reason)
        };
        drop_action.requires_confirmation = true;
        drop_action.irreversible = true;
        actions.push(drop_action);

        // SPEC-091 IW2 — full legacy vector fleet drop (migration 131).
        let mut fleet_drop = if v.fleet_retirable() {
            GuardedAction::enabled("drop", "vector-fleet")
        } else {
            let reason = if !matches!(v.backend.as_str(), "typed_embeddings" | "chunk_embeddings") {
                format!(
                    "cannot drop vector-fleet: backend is '{}' (flip to typed_embeddings first)",
                    v.backend
                )
            } else if v.uncovered_fleet_rows > 0 {
                let stall_hint = if v.provenance_stall_rows > 0 {
                    format!(
                        " ({} dual-legacy stall(s): typed row already holds a different \
                         legacy_vector_id — inspect/delete alias residue; no auto-delete)",
                        v.provenance_stall_rows
                    )
                } else {
                    String::new()
                };
                format!(
                    "cannot drop vector-fleet: {} legacy fleet rows lack legacy_vector_id provenance \
                     — run iw2-fleet-embedding-backfill or iw2-fleet-provenance-stamp{}",
                    v.uncovered_fleet_rows, stall_hint
                )
            } else if v.uncovered_chunk_rows > 0 {
                format!(
                    "cannot drop vector-fleet: {} legacy chunk rows uncovered \
                     (missing_spine={}, missing_embedding={}; migration 126 first)",
                    v.uncovered_chunk_rows,
                    v.uncovered_chunk_missing_spine_rows,
                    v.uncovered_chunk_missing_embedding_rows
                )
            } else if !v.verify_fleet.map(|x| x.passes()).unwrap_or(false) {
                "cannot drop vector-fleet: fleet verify not passing".to_string()
            } else {
                "cannot drop vector-fleet: not ready".to_string()
            };
            GuardedAction::gated("drop", "vector-fleet", reason)
        };
        fleet_drop.requires_confirmation = true;
        fleet_drop.irreversible = true;
        actions.push(fleet_drop);

        if v.uncovered_fleet_rows > 0 {
            actions.push(GuardedAction::enabled("run", "iw2-fleet-provenance-stamp"));
        }
    }

    actions
}

/// Format seconds as an HH:MM:SS ETA.
fn fmt_eta(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("ETA {:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration_engine::advisor::types::{
        FamilySpec, ResidueReport, VectorPosture, FAMILIES,
    };

    fn spec(name: &str) -> FamilySpec {
        FAMILIES.iter().copied().find(|s| s.name == name).unwrap()
    }

    fn job(state: &str, pct: Option<f64>) -> JobSnapshot {
        JobSnapshot {
            step_id: "w1-chunk-text-backfill".to_string(),
            job_id: Some("00000000-0000-0000-0000-0000000000aa".to_string()),
            state: state.to_string(),
            completion_pct: pct,
            processed_count: 0,
            estimated_total: Some(100),
            rows_per_sec: None,
            eta_seconds: None,
            throttle_reason: None,
            last_error: None,
        }
    }

    fn family(
        name: &str,
        mode: FamilyMode,
        phase: FamilyPhase,
        residue: i64,
        backfill: Option<JobSnapshot>,
        verify: Option<VerifySummary>,
    ) -> FamilyPosture {
        let s = spec(name);
        FamilyPosture {
            family: s.name,
            mode,
            phase,
            durable: s.durable,
            backfill,
            verify,
            kv_residue_rows: residue,
            typed_rows: 0,
            typed_tables: s.typed_tables,
            env_flag: s.env_flag,
        }
    }

    fn posture(
        kv_dropped: bool,
        engine_installed: bool,
        families: Vec<FamilyPosture>,
        residue: ResidueReport,
    ) -> MigrationPosture {
        let mut p = MigrationPosture {
            kv_store_dropped: kv_dropped,
            engine_installed,
            engine_mode: MigrationMode::Automatic,
            serving_fence_on: true,
            families,
            residue,
            vector: VectorPosture {
                backend: "legacy_tables".to_string(),
                backfill: None,
                fleet_backfill: None,
                verify_chunk: None,
                verify_fleet: None,
                verify: None,
                provenance_stall_rows: 0,
                typed_rows: 0,
                typed_entity_rows: 0,
                typed_relationship_rows: 0,
                typed_report_rows: 0,
                legacy_chunk_rows: 0,
                legacy_fleet_rows: 0,
                uncovered_chunk_rows: 0,
                uncovered_chunk_missing_spine_rows: 0,
                uncovered_chunk_missing_embedding_rows: 0,
                uncovered_fleet_rows: 0,
                chunk_fleet_dropped: false,
                dropped: false,
            },
            cutover_phase: CutoverPhase::InProgress,
        };
        p.cutover_phase = derive_cutover_phase(&p);
        p
    }

    // ---- derive_family_phase over every phase ----------------------------

    #[test]
    fn contract_spec091_phase_not_started() {
        assert_eq!(
            derive_family_phase(FamilyMode::Kv, false, None, None, 5, false, false),
            FamilyPhase::NotStarted
        );
    }

    #[test]
    fn contract_spec091_phase_dual_writing() {
        assert_eq!(
            derive_family_phase(FamilyMode::Dual, true, None, None, 3, false, false),
            FamilyPhase::DualWriting
        );
    }

    #[test]
    fn contract_spec091_phase_backfilling_and_verifying() {
        let running = job("running", Some(42.0));
        assert_eq!(
            derive_family_phase(
                FamilyMode::Dual,
                true,
                Some(&running),
                None,
                3,
                false,
                false
            ),
            FamilyPhase::Backfilling
        );
        let paused = job("paused", Some(40.0));
        assert_eq!(
            derive_family_phase(FamilyMode::Dual, true, Some(&paused), None, 3, false, false),
            FamilyPhase::Backfilling
        );
        let verifying = job("verifying", Some(100.0));
        assert_eq!(
            derive_family_phase(
                FamilyMode::Dual,
                true,
                Some(&verifying),
                None,
                0,
                false,
                false
            ),
            FamilyPhase::Verifying
        );
    }

    #[test]
    fn contract_spec091_phase_ready_to_flip_chunk_requires_verify() {
        let done = job("completed", Some(100.0));
        let uncovered = VerifySummary {
            expected: 10,
            actual: 9,
            sampled: 5,
            mismatches: 0,
        };
        assert_eq!(
            derive_family_phase(
                FamilyMode::Dual,
                true,
                Some(&done),
                Some(&uncovered),
                0,
                false,
                false
            ),
            FamilyPhase::DualWriting
        );
        let mismatches_ok = VerifySummary {
            expected: 10,
            actual: 10,
            sampled: 5,
            mismatches: 2,
        };
        assert_eq!(
            derive_family_phase(
                FamilyMode::Dual,
                true,
                Some(&done),
                Some(&mismatches_ok),
                0,
                false,
                false
            ),
            FamilyPhase::ReadyToFlip,
            "LAW-139: sampled mismatches do not block flip (coverage-first)"
        );
        let good = VerifySummary {
            expected: 10,
            actual: 10,
            sampled: 5,
            mismatches: 0,
        };
        assert_eq!(
            derive_family_phase(
                FamilyMode::Dual,
                true,
                Some(&done),
                Some(&good),
                0,
                false,
                false
            ),
            FamilyPhase::ReadyToFlip
        );
    }

    #[test]
    fn contract_spec091_phase_ready_to_flip_non_chunk_residue_only() {
        assert_eq!(
            derive_family_phase(FamilyMode::Kv, false, None, None, 0, false, false),
            FamilyPhase::ReadyToFlip
        );
    }

    #[test]
    fn contract_spec091_phase_flipped_ready_to_drop_dropped() {
        assert_eq!(
            derive_family_phase(FamilyMode::Relational, false, None, None, 0, false, false),
            FamilyPhase::Flipped
        );
        assert_eq!(
            derive_family_phase(FamilyMode::Relational, false, None, None, 0, false, true),
            FamilyPhase::ReadyToDrop
        );
        assert_eq!(
            derive_family_phase(FamilyMode::Dual, true, None, None, 5, true, false),
            FamilyPhase::Dropped
        );
    }

    // ---- aggregate cutover phase -----------------------------------------

    #[test]
    fn contract_spec091_cutover_phase_rollup() {
        let fresh = posture(
            false,
            true,
            vec![
                family(
                    "CHUNK",
                    FamilyMode::Kv,
                    FamilyPhase::NotStarted,
                    10,
                    None,
                    None,
                ),
                family(
                    "METADATA",
                    FamilyMode::Kv,
                    FamilyPhase::NotStarted,
                    4,
                    None,
                    None,
                ),
            ],
            ResidueReport {
                chunk_text: 10,
                doc_shells: 4,
                ..Default::default()
            },
        );
        assert_eq!(fresh.cutover_phase, CutoverPhase::NotStarted);

        let ready = posture(
            false,
            true,
            vec![
                family(
                    "CHUNK",
                    FamilyMode::Relational,
                    FamilyPhase::Flipped,
                    0,
                    None,
                    None,
                ),
                family(
                    "METADATA",
                    FamilyMode::Relational,
                    FamilyPhase::Flipped,
                    0,
                    None,
                    None,
                ),
            ],
            ResidueReport::default(),
        );
        assert_eq!(ready.cutover_phase, CutoverPhase::ReadyToDrop);

        let dropped = posture(
            true,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Relational,
                FamilyPhase::Dropped,
                0,
                None,
                None,
            )],
            ResidueReport::default(),
        );
        assert_eq!(dropped.cutover_phase, CutoverPhase::Dropped);
    }

    // ---- guidance: the decision table ------------------------------------

    #[test]
    fn contract_spec091_guidance_waits_while_backfilling() {
        let p = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::Backfilling,
                501,
                Some(JobSnapshot {
                    eta_seconds: Some(4364.0),
                    ..job("running", Some(42.3))
                }),
                None,
            )],
            ResidueReport {
                chunk_text: 501,
                ..Default::default()
            },
        );
        let g = derive_guidance(&p);
        let wait = g
            .instructions
            .iter()
            .find(|i| i.kind == InstrKind::Wait)
            .unwrap();
        assert!(wait.summary.contains("WAIT"));
        assert!(wait.summary.contains("CHUNK"));
        assert!(wait.summary.contains("Do not flip"));
    }

    #[test]
    fn contract_spec091_guidance_ready_to_flip_action() {
        let verify = VerifySummary {
            expected: 10,
            actual: 10,
            sampled: 5,
            mismatches: 0,
        };
        let p = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::ReadyToFlip,
                0,
                Some(job("completed", Some(100.0))),
                Some(verify),
            )],
            ResidueReport::default(),
        );
        let g = derive_guidance(&p);
        let action = g
            .instructions
            .iter()
            .find(|i| i.kind == InstrKind::Action)
            .unwrap();
        assert!(action
            .summary
            .contains("EDGEQUAKE_CHUNK_TEXT_AUTHORITY=relational"));
        assert_eq!(
            action.command.as_deref(),
            Some("export EDGEQUAKE_CHUNK_TEXT_AUTHORITY=relational")
        );
    }

    #[test]
    fn contract_spec091_guidance_stale_flag_post_drop_blocked() {
        // EC-C1: a dual flag against a dropped store must be BLOCKED.
        let p = posture(
            true,
            true,
            vec![
                family(
                    "CHUNK",
                    FamilyMode::Dual,
                    FamilyPhase::Dropped,
                    0,
                    None,
                    None,
                ),
                family(
                    "METADATA",
                    FamilyMode::Relational,
                    FamilyPhase::Dropped,
                    0,
                    None,
                    None,
                ),
            ],
            ResidueReport::default(),
        );
        let g = derive_guidance(&p);
        let blocked: Vec<_> = g
            .instructions
            .iter()
            .filter(|i| i.kind == InstrKind::Blocked)
            .collect();
        assert!(blocked.iter().any(|i| i.summary.contains("42P01")));
        assert!(blocked.iter().any(|i| i.summary.contains("DROPPED")));
        // Rollback to kv must be refused post-drop.
        let actions = derive_actions(&p);
        assert!(!actions
            .iter()
            .any(|a| a.verb == "family.set kv" && a.target == "CHUNK" && a.enabled));
    }

    #[test]
    fn contract_spec091_actions_ec_c1_post_drop_reachable() {
        // EC-C1 guardrail must be REACHABLE (regression for the dead-code bug
        // where phase==Dropped suppressed every family action): a relational
        // family post-drop offers a GATED set→kv; a stale dual family offers an
        // ENABLED set→relational (the remedy).
        let p = posture(
            true,
            true,
            vec![
                family(
                    "CHUNK",
                    FamilyMode::Dual,
                    FamilyPhase::Dropped,
                    0,
                    None,
                    None,
                ),
                family(
                    "METADATA",
                    FamilyMode::Relational,
                    FamilyPhase::Dropped,
                    0,
                    None,
                    None,
                ),
            ],
            ResidueReport::default(),
        );
        let a = derive_actions(&p);
        let kv = a
            .iter()
            .find(|x| x.verb == "family.set kv" && x.target == "METADATA")
            .expect("relational family post-drop must offer the gated rollback");
        assert!(!kv.enabled);
        assert!(kv.gate_reason.as_deref().unwrap().contains("42P01"));
        let rel = a
            .iter()
            .find(|x| x.verb == "family.set relational" && x.target == "CHUNK")
            .expect("stale dual family post-drop must offer the remedy");
        assert!(rel.enabled);
    }

    #[test]
    fn contract_spec091_guidance_done_when_all_relational_post_drop() {
        let p = posture(
            true,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Relational,
                FamilyPhase::Dropped,
                0,
                None,
                None,
            )],
            ResidueReport::default(),
        );
        let g = derive_guidance(&p);
        assert!(g.instructions.iter().any(|i| i.kind == InstrKind::Done));
    }

    #[test]
    fn contract_spec091_guidance_engine_off_blocked() {
        let mut p = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::DualWriting,
                5,
                None,
                None,
            )],
            ResidueReport {
                chunk_text: 5,
                ..Default::default()
            },
        );
        p.engine_mode = MigrationMode::Off;
        let g = derive_guidance(&p);
        assert!(g.instructions.iter().any(
            |i| i.kind == InstrKind::Blocked && i.summary.contains("EDGEQUAKE_MIGRATION_MODE")
        ));
    }

    #[test]
    fn contract_spec091_guidance_no_ledger_blocked() {
        let p = posture(
            false,
            false,
            vec![family(
                "CHUNK",
                FamilyMode::Kv,
                FamilyPhase::NotStarted,
                5,
                None,
                None,
            )],
            ResidueReport {
                chunk_text: 5,
                ..Default::default()
            },
        );
        let g = derive_guidance(&p);
        assert!(g
            .instructions
            .iter()
            .any(|i| i.kind == InstrKind::Blocked && i.summary.contains("engine ledger")));
    }

    #[test]
    fn contract_spec091_guidance_confirm_when_ready_to_drop() {
        let p = posture(
            false,
            true,
            vec![
                family(
                    "CHUNK",
                    FamilyMode::Relational,
                    FamilyPhase::Flipped,
                    0,
                    None,
                    None,
                ),
                family(
                    "METADATA",
                    FamilyMode::Relational,
                    FamilyPhase::Flipped,
                    0,
                    None,
                    None,
                ),
            ],
            ResidueReport::default(),
        );
        assert_eq!(p.cutover_phase, CutoverPhase::ReadyToDrop);
        let g = derive_guidance(&p);
        let confirm = g
            .instructions
            .iter()
            .find(|i| i.kind == InstrKind::Confirm)
            .unwrap();
        assert!(confirm.summary.contains("--confirm-drop"));
        assert!(confirm.summary.contains("IRREVERSIBLE"));
        let actions = derive_actions(&p);
        let drop = actions.iter().find(|a| a.verb == "drop").unwrap();
        assert!(drop.enabled && drop.requires_confirmation && drop.irreversible);
    }

    #[test]
    fn contract_spec091_guidance_drop_blocked_with_residue() {
        let p = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Relational,
                FamilyPhase::Flipped,
                3,
                None,
                None,
            )],
            ResidueReport {
                chunk_text: 3,
                ..Default::default()
            },
        );
        let g = derive_guidance(&p);
        assert!(g
            .instructions
            .iter()
            .any(|i| i.kind == InstrKind::Blocked && i.summary.contains("durable KV rows")));
        let actions = derive_actions(&p);
        let drop = actions.iter().find(|a| a.verb == "drop").unwrap();
        assert!(!drop.enabled);
    }

    #[test]
    fn contract_spec091_actions_family_set_relational_gated_until_ready() {
        let not_ready = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::DualWriting,
                5,
                None,
                None,
            )],
            ResidueReport {
                chunk_text: 5,
                ..Default::default()
            },
        );
        let a = derive_actions(&not_ready);
        let set = a
            .iter()
            .find(|x| x.verb == "family.set relational" && x.target == "CHUNK")
            .unwrap();
        assert!(!set.enabled && set.gate_reason.is_some());

        let verify = VerifySummary {
            expected: 0,
            actual: 0,
            sampled: 0,
            mismatches: 0,
        };
        let ready = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::ReadyToFlip,
                0,
                Some(job("completed", Some(100.0))),
                Some(verify),
            )],
            ResidueReport::default(),
        );
        let a = derive_actions(&ready);
        let set = a
            .iter()
            .find(|x| x.verb == "family.set relational" && x.target == "CHUNK")
            .unwrap();
        assert!(set.enabled);
    }

    #[test]
    fn contract_spec091_actions_job_control_mirror_lease() {
        let p = posture(
            false,
            true,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::Backfilling,
                5,
                Some(job("running", Some(10.0))),
                None,
            )],
            ResidueReport::default(),
        );
        let a = derive_actions(&p);
        let by = |v: &str| a.iter().find(|x| x.verb == v).unwrap();
        assert!(by("pause").enabled);
        assert!(by("cancel").enabled);
        assert!(!by("resume").enabled);
    }

    #[test]
    fn contract_spec091_instruction_ordinals_are_sequential() {
        let p = posture(
            false,
            false,
            vec![family(
                "CHUNK",
                FamilyMode::Dual,
                FamilyPhase::DualWriting,
                5,
                None,
                None,
            )],
            ResidueReport {
                chunk_text: 5,
                ..Default::default()
            },
        );
        let g = derive_guidance(&p);
        for (i, ins) in g.instructions.iter().enumerate() {
            assert_eq!(ins.ordinal, (i + 1) as u32);
        }
    }
}
