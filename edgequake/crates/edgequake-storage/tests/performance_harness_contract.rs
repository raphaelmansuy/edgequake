//! Guard the measurement gates themselves; these tests need no database.
#[path = "support/perf_harness.rs"]
mod perf_harness;

use perf_harness::{emit_documented, finish_report, percentile_p95_ms, samples_after_warmup};
use std::time::Duration;

#[test]
fn p95_uses_nearest_rank_on_unsorted_samples() {
    let samples: Vec<_> = (1..=100).rev().map(Duration::from_millis).collect();
    assert_eq!(percentile_p95_ms(&samples), 95.0);
    assert_eq!(samples_after_warmup(&samples, 30).len(), 99);
    assert_eq!(samples_after_warmup(&samples[..30], 30).len(), 30);
}

#[test]
fn report_preserves_samples_and_count() {
    let samples = vec![Duration::from_millis(2); 30];
    let report = finish_report("probe", &samples, 10.0, "round_trip", false, "");
    let json: serde_json::Value = serde_json::from_str(&report.to_json_line()).unwrap();
    assert_eq!(json["sample_count"], 30);
    assert_eq!(json["samples_ms"].as_array().unwrap().len(), 30);
    assert_eq!(json["p95_ms"], 2.0);
    assert_eq!(json["pass"], true);
}

#[test]
fn empty_measurements_cannot_pass_any_report_path() {
    assert!(std::panic::catch_unwind(|| finish_report("empty", &[], 10.0, "", false, "")).is_err());
    assert!(std::panic::catch_unwind(|| emit_documented("empty", &[], "", "")).is_err());
}

#[test]
fn invalid_budgets_and_slow_samples_fail() {
    let samples = [Duration::from_millis(10)];
    for budget in [0.0, -1.0, f64::NAN, f64::INFINITY, 10.0, 9.0] {
        assert!(std::panic::catch_unwind(|| finish_report(
            "invalid", &samples, budget, "", false, ""
        ))
        .is_err());
    }
}
