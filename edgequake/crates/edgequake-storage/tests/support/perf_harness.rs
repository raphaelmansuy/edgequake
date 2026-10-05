//! SPEC-061/062 — shared DataAccess performance helpers (DRY across e2e gates).
#![allow(dead_code)]

use serde_json::json;
use std::time::Duration;

/// Default sample floor for SPEC-062 write/read hygiene (after optional warmup drop).
pub const PERF_SAMPLE_FLOOR: usize = 30;

/// Drop the first sample (cold/warmup). Prefer `min_keep` remaining when available.
pub fn samples_after_warmup(samples: &[Duration], min_keep: usize) -> Vec<Duration> {
    let out: Vec<Duration> = if samples.len() > 1 {
        samples[1..].to_vec()
    } else {
        samples.to_vec()
    };
    if out.len() < min_keep && samples.len() >= min_keep {
        return samples.to_vec();
    }
    out
}

/// Emit a pass/fail report without asserting a numeric budget (for documented walls).
pub fn emit_documented(
    op: &str,
    samples: &[Duration],
    plan_class: &str,
    detail: impl Into<String>,
) -> PerfReport {
    assert!(
        !samples.is_empty(),
        "{op}: no performance samples collected"
    );
    let p95_ms = percentile_p95_ms(samples);
    let report = PerfReport {
        op: op.to_string(),
        p95_ms,
        samples: durations_to_ms(samples),
        plan_class: plan_class.to_string(),
        buffers_hint: false,
        pass: true,
        detail: detail.into(),
    };
    report.emit();
    report
}

/// Plan classes we accept on hot paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanKind {
    Hnsw,
    Gin,
    Btree,
    Bitmap,
    Index, // any Index Scan / Index Only Scan
}

/// One measured op result (JSONL-friendly).
#[derive(Debug, Clone)]
pub struct PerfReport {
    pub op: String,
    pub p95_ms: f64,
    pub samples: Vec<f64>,
    pub plan_class: String,
    pub buffers_hint: bool,
    pub pass: bool,
    pub detail: String,
}

impl PerfReport {
    pub fn to_json_line(&self) -> String {
        let profile = std::env::var("EQ_POSTGRES_PROFILE").unwrap_or_else(|_| "unknown".into());
        let mut obj = json!({
            "profile": profile,
            "pg_major": std::env::var("EQ_POSTGRES_MAJOR").unwrap_or_default(),
            "op": self.op,
            "p95_ms": self.p95_ms,
            "samples_ms": self.samples,
            "sample_count": self.samples.len(),
            "plan_class": self.plan_class,
            "buffers_hint": self.buffers_hint,
            "pass": self.pass,
            "detail": self.detail,
        });
        // SPEC-062: annotate known noisy ops so cross-major 2× gate can allowlist.
        if self.detail.contains("noise_ok") {
            obj["noise_ok"] = json!(true);
        }
        obj.to_string()
    }

    /// Emit one JSONL line to stdout (CI scrapes → `/tmp/eq-perf-{profile}.jsonl`).
    pub fn emit(&self) {
        println!("PERF_REPORT {}", self.to_json_line());
    }
}

pub fn percentile_p95(sorted: &[Duration]) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
    sorted[idx.min(sorted.len() - 1)]
}

pub fn percentile_p95_ms(samples: &[Duration]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort();
    percentile_p95(&sorted).as_secs_f64() * 1000.0
}

pub fn durations_to_ms(samples: &[Duration]) -> Vec<f64> {
    samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect()
}

/// Assert EXPLAIN text uses an index-like path for the given kinds.
pub fn assert_plan_uses_index(plan: &str, kinds: &[PlanKind]) {
    let lower = plan.to_lowercase();
    let mut ok = false;
    let mut matched = Vec::new();
    for k in kinds {
        let hit = match k {
            PlanKind::Hnsw => lower.contains("hnsw"),
            PlanKind::Gin => lower.contains("gin") || lower.contains("bitmap index scan"),
            PlanKind::Btree => {
                lower.contains("btree") || (lower.contains("index scan") && !lower.contains("hnsw"))
            }
            PlanKind::Bitmap => lower.contains("bitmap"),
            PlanKind::Index => {
                lower.contains("index scan")
                    || lower.contains("index only scan")
                    || lower.contains("bitmap index")
            }
        };
        if hit {
            ok = true;
            matched.push(format!("{k:?}"));
        }
    }
    assert!(
        ok,
        "EXPLAIN must use {:?} path; matched={matched:?}; plan was:\n{plan}",
        kinds
    );
    assert!(
        !lower.contains("seq scan") || lower.contains("index"),
        "EXPLAIN must not be a plain Seq Scan on hot path; plan was:\n{plan}"
    );
}

pub fn plan_has_buffers(plan: &str) -> bool {
    let lower = plan.to_lowercase();
    lower.contains("buffers:") || lower.contains("shared")
}

/// Build + emit a PerfReport; panic if `pass` is false.
pub fn finish_report(
    op: &str,
    samples: &[Duration],
    budget_ms: f64,
    plan_class: &str,
    buffers_hint: bool,
    detail: impl Into<String>,
) -> PerfReport {
    assert!(
        !samples.is_empty(),
        "{op}: no performance samples collected"
    );
    assert!(
        budget_ms.is_finite() && budget_ms > 0.0,
        "{op}: performance budget must be finite and positive"
    );
    let p95_ms = percentile_p95_ms(samples);
    let pass = p95_ms < budget_ms;
    let report = PerfReport {
        op: op.to_string(),
        p95_ms,
        samples: durations_to_ms(samples),
        plan_class: plan_class.to_string(),
        buffers_hint,
        pass,
        detail: detail.into(),
    };
    report.emit();
    assert!(
        pass,
        "{} FAIL: p95 {:.2}ms exceeds budget {:.2}ms (samples_ms={:?})",
        op, p95_ms, budget_ms, report.samples
    );
    report
}

/// Join EXPLAIN rows into a single plan string.
pub fn join_plan_rows(rows: Vec<(String,)>) -> String {
    rows.into_iter().map(|r| r.0).collect::<Vec<_>>().join("\n")
}
