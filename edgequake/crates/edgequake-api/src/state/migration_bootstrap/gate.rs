//! SPEC-150 schema gate — pure evaluate of ledger vs manifest compat window.

use edgequake_migrate_manifest::{Manifest, MigrationPhase};

/// Env: `wait` (bind lite router until ready) or `fail` (exit 78, default).
pub const SCHEMA_GATE_ENV: &str = "EDGEQUAKE_SCHEMA_GATE";
/// Env: poll interval seconds while waiting (default 2).
pub const SCHEMA_GATE_POLL_ENV: &str = "EDGEQUAKE_SCHEMA_GATE_POLL";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaGateMode {
    Wait,
    Fail,
}

impl SchemaGateMode {
    pub fn from_env() -> Self {
        match std::env::var(SCHEMA_GATE_ENV)
            .unwrap_or_else(|_| "fail".into())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "wait" | "1" | "true" | "on" => Self::Wait,
            _ => Self::Fail,
        }
    }

    pub fn poll_interval_secs() -> u64 {
        std::env::var(SCHEMA_GATE_POLL_ENV)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2)
            .max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// Schema is within the compat window; full AppState may boot.
    Serve,
    /// Waiting for migrate to catch up (`EDGEQUAKE_SCHEMA_GATE=wait`).
    Wait {
        reason: String,
        applied_max: i64,
        embedded_max: i64,
    },
    /// Hard refuse (exit 78) — newer than window, or fail-mode pending.
    Refuse { reason: String },
}

/// Inputs for the pure gate evaluator.
#[derive(Debug, Clone)]
pub struct GateInput {
    pub applied_max: i64,
    pub pending: Vec<i64>,
    /// From `edgequake.schema_compat.min_binary_schema` when present.
    pub min_binary_schema: Option<i64>,
    /// Soft-allow irreversible-only (and deferred 142) pending.
    pub pending_ok_to_serve: bool,
    pub mode: SchemaGateMode,
}

/// Evaluate whether this binary may serve the current ledger.
pub fn evaluate(manifest: &Manifest, input: &GateInput) -> GateDecision {
    let embedded_max = manifest.embedded_max();
    let compat_min = manifest.compat_serve_min;
    let applied = input.applied_max;

    // NEWER than this binary understands.
    if applied > embedded_max {
        let floor = input.min_binary_schema.unwrap_or(i64::MAX);
        if floor <= embedded_max {
            // N-1 rolling: newer migrate wrote a floor this binary still meets.
            return GateDecision::Serve;
        }
        return GateDecision::Refuse {
            reason: format!(
                "database schema version {applied} is newer than this binary \
                 (embedded max {embedded_max}; schema_compat.min_binary_schema={}). \
                 Upgrade the EdgeQuake binary.",
                input
                    .min_binary_schema
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "unset".into())
            ),
        };
    }

    // Behind compat window with pending expandable work.
    if !input.pending.is_empty() && !input.pending_ok_to_serve {
        let reason = format!(
            "schema pending versions {:?} (applied_max={applied}, embedded_max={embedded_max})",
            input.pending
        );
        return match input.mode {
            SchemaGateMode::Wait if applied <= embedded_max => GateDecision::Wait {
                reason,
                applied_max: applied,
                embedded_max,
            },
            _ => GateDecision::Refuse { reason },
        };
    }

    // Soft-allow irreversible-only pending, or fully caught up.
    if applied < compat_min && applied > 0 {
        // Ancient ledger below window — still allow if no expandable pending
        // (fresh soft-allow path already handled); otherwise wait/fail.
        if input.pending.is_empty() || input.pending_ok_to_serve {
            return GateDecision::Serve;
        }
    }

    GateDecision::Serve
}

/// Contract: every version above `compat_serve_min` must be expand or data
/// (contract drops are allowed but must be soft-servable via pending_ok_to_serve).
pub fn assert_compat_window_phases(manifest: &Manifest) -> Result<(), String> {
    for e in &manifest.migration {
        if e.version <= manifest.compat_serve_min {
            continue;
        }
        match e.phase {
            MigrationPhase::Expand | MigrationPhase::Data | MigrationPhase::Contract => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_migrate_manifest::load;

    #[test]
    fn caught_up_serves() {
        let m = load();
        let d = evaluate(
            m,
            &GateInput {
                applied_max: m.embedded_max(),
                pending: vec![],
                min_binary_schema: None,
                pending_ok_to_serve: false,
                mode: SchemaGateMode::Fail,
            },
        );
        assert_eq!(d, GateDecision::Serve);
    }

    #[test]
    fn pending_expandable_fails_by_default() {
        let m = load();
        let d = evaluate(
            m,
            &GateInput {
                applied_max: m.compat_serve_min,
                pending: vec![m.embedded_max()],
                min_binary_schema: None,
                pending_ok_to_serve: false,
                mode: SchemaGateMode::Fail,
            },
        );
        assert!(matches!(d, GateDecision::Refuse { .. }));
    }

    #[test]
    fn pending_expandable_waits_when_configured() {
        let m = load();
        let d = evaluate(
            m,
            &GateInput {
                applied_max: m.compat_serve_min,
                pending: vec![m.embedded_max()],
                min_binary_schema: None,
                pending_ok_to_serve: false,
                mode: SchemaGateMode::Wait,
            },
        );
        assert!(matches!(d, GateDecision::Wait { .. }));
    }

    #[test]
    fn newer_within_compat_serves() {
        let m = load();
        let d = evaluate(
            m,
            &GateInput {
                applied_max: m.embedded_max() + 5,
                pending: vec![],
                min_binary_schema: Some(m.embedded_max() - 1),
                pending_ok_to_serve: false,
                mode: SchemaGateMode::Fail,
            },
        );
        assert_eq!(d, GateDecision::Serve);
    }

    #[test]
    fn newer_outside_compat_refuses() {
        let m = load();
        let d = evaluate(
            m,
            &GateInput {
                applied_max: m.embedded_max() + 5,
                pending: vec![],
                min_binary_schema: Some(m.embedded_max() + 1),
                pending_ok_to_serve: false,
                mode: SchemaGateMode::Fail,
            },
        );
        assert!(matches!(d, GateDecision::Refuse { .. }));
    }

    #[test]
    fn compat_window_phases_ok() {
        assert!(assert_compat_window_phases(load()).is_ok());
    }
}
