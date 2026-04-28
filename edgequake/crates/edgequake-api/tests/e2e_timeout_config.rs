//! End-to-end tests for configurable chunk-extraction timeout & concurrency.
//!
//! ## Why these tests exist
//!
//! Issue #194 revealed that `chunk_extraction_timeout_secs` (180 s) and
//! `max_concurrent_extractions` (16) were hardcoded with NO env-var override.
//! Users with slow local Ollama instances saw:
//!
//! ```text
//! Timeout after 180s (attempt 3/3) — All 1 chunks failed extraction.
//! ```
//!
//! This test suite verifies:
//!
//! 1. `PipelineConfig::from_env()` reads all four tuning env vars.
//! 2. Missing env vars fall back to compile-time defaults (no breaking change).
//! 3. Values below the minimum are clamped (e.g. `EDGEQUAKE_CHUNK_TIMEOUT_SECS=1` → 10 s).
//! 4. Non-numeric values are silently ignored (default used).
//! 5. All four env vars interact correctly when set together.
//! 6. `Pipeline::default_pipeline()` reflects the env vars at construction time.
//! 7. `SafetyLimitsConfig` now accepts up to 3600 s (`MAXIMUM_TIMEOUT_SECS` raised).
//!
//! @implements SPEC-010-T/FR-T01 through FR-T07
//! @implements SPEC-010-T/NFR-T02

use edgequake_pipeline::{
    PipelineConfig, DEFAULT_CHUNK_MAX_RETRIES, DEFAULT_CHUNK_TIMEOUT_SECS,
    DEFAULT_INITIAL_RETRY_DELAY_MS, DEFAULT_MAX_CONCURRENT_EXTRACTIONS, MAX_CHUNK_MAX_RETRIES,
    MIN_CHUNK_TIMEOUT_SECS,
};
use serial_test::serial;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Guard that removes env vars on drop so tests don't pollute each other.
struct EnvGuard(Vec<&'static str>);

impl EnvGuard {
    fn set(vars: &[(&'static str, &str)]) -> Self {
        for (k, v) in vars {
            // SAFETY: single-threaded test — each test uses its own guard
            unsafe { std::env::set_var(k, v) };
        }
        Self(vars.iter().map(|(k, _)| *k).collect())
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for k in &self.0 {
            unsafe { std::env::remove_var(k) };
        }
    }
}

/// All four configurable env var names.
const CHUNK_TIMEOUT_VAR: &str = "EDGEQUAKE_CHUNK_TIMEOUT_SECS";
const MAX_RETRIES_VAR: &str = "EDGEQUAKE_CHUNK_MAX_RETRIES";
const RETRY_DELAY_VAR: &str = "EDGEQUAKE_CHUNK_RETRY_DELAY_MS";
const MAX_CONCURRENT_VAR: &str = "EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS";

// ─────────────────────────────────────────────────────────────────────────────
// Test 1 — Defaults when no env vars are set
// ─────────────────────────────────────────────────────────────────────────────

/// Proves that `from_env()` returns compile-time defaults when no env vars
/// are present.  This is the "no breaking change" guarantee.
#[test]
#[serial]
fn test_from_env_returns_defaults_when_no_env_vars_set() {
    // Ensure none of the 4 vars are set
    let _guard = EnvGuard::set(&[]);
    unsafe {
        std::env::remove_var(CHUNK_TIMEOUT_VAR);
        std::env::remove_var(MAX_RETRIES_VAR);
        std::env::remove_var(RETRY_DELAY_VAR);
        std::env::remove_var(MAX_CONCURRENT_VAR);
    }

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_extraction_timeout_secs, DEFAULT_CHUNK_TIMEOUT_SECS,
        "chunk timeout must match default when env var absent"
    );
    assert_eq!(
        config.chunk_max_retries, DEFAULT_CHUNK_MAX_RETRIES,
        "max retries must match default when env var absent"
    );
    assert_eq!(
        config.initial_retry_delay_ms, DEFAULT_INITIAL_RETRY_DELAY_MS,
        "retry delay must match default when env var absent"
    );
    assert_eq!(
        config.max_concurrent_extractions, DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
        "max concurrent must match default when env var absent"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — Chunk timeout env var is read
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_CHUNK_TIMEOUT_SECS=600` produces a 600 s timeout.
/// This is the primary fix for issue #194.
#[test]
#[serial]
fn test_chunk_timeout_env_var_overrides_default() {
    let _guard = EnvGuard::set(&[(CHUNK_TIMEOUT_VAR, "600")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_extraction_timeout_secs, 600,
        "chunk timeout env var (600) must be respected"
    );
    // Other fields must stay at defaults
    assert_eq!(config.chunk_max_retries, DEFAULT_CHUNK_MAX_RETRIES);
    assert_eq!(
        config.max_concurrent_extractions,
        DEFAULT_MAX_CONCURRENT_EXTRACTIONS
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — Max retries env var is read
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_CHUNK_MAX_RETRIES=1` produces a single retry attempt.
#[test]
#[serial]
fn test_max_retries_env_var_overrides_default() {
    let _guard = EnvGuard::set(&[(MAX_RETRIES_VAR, "1")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_max_retries, 1,
        "max retries env var (1) must be respected"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — Retry delay env var is read
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_CHUNK_RETRY_DELAY_MS=5000` sets the initial backoff to 5 s.
#[test]
#[serial]
fn test_retry_delay_env_var_overrides_default() {
    let _guard = EnvGuard::set(&[(RETRY_DELAY_VAR, "5000")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.initial_retry_delay_ms, 5000,
        "retry delay env var (5000) must be respected"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — Max concurrent env var is read
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4` reduces parallelism.
/// This addresses the Ollama overload cascade described in issue #194.
#[test]
#[serial]
fn test_max_concurrent_env_var_overrides_default() {
    let _guard = EnvGuard::set(&[(MAX_CONCURRENT_VAR, "4")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.max_concurrent_extractions, 4,
        "max concurrent env var (4) must be respected"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6 — All four env vars set together
// ─────────────────────────────────────────────────────────────────────────────

/// Proves all four vars can be set simultaneously without interference.
/// Mirrors the recommended `.env` for a slow local Ollama instance.
#[test]
#[serial]
fn test_all_four_env_vars_set_together() {
    let _guard = EnvGuard::set(&[
        (CHUNK_TIMEOUT_VAR, "600"),
        (MAX_RETRIES_VAR, "2"),
        (RETRY_DELAY_VAR, "2000"),
        (MAX_CONCURRENT_VAR, "4"),
    ]);

    let config = PipelineConfig::from_env();

    assert_eq!(config.chunk_extraction_timeout_secs, 600);
    assert_eq!(config.chunk_max_retries, 2);
    assert_eq!(config.initial_retry_delay_ms, 2000);
    assert_eq!(config.max_concurrent_extractions, 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7 — Minimum clamping: value below minimum is clamped
// ─────────────────────────────────────────────────────────────────────────────

/// Proves that `EDGEQUAKE_CHUNK_TIMEOUT_SECS=1` (below MIN_CHUNK_TIMEOUT_SECS=10)
/// is clamped to the minimum rather than producing a nonsensical sub-second timeout.
#[test]
#[serial]
fn test_chunk_timeout_below_minimum_is_clamped() {
    let _guard = EnvGuard::set(&[(CHUNK_TIMEOUT_VAR, "1")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_extraction_timeout_secs, MIN_CHUNK_TIMEOUT_SECS,
        "timeout below minimum must be clamped to MIN_CHUNK_TIMEOUT_SECS"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 8 — Non-numeric value is ignored (default used)
// ─────────────────────────────────────────────────────────────────────────────

/// Proves that `EDGEQUAKE_CHUNK_TIMEOUT_SECS=abc` is silently ignored and the
/// default is used.  This prevents startup panics on misconfigured deployments.
#[test]
#[serial]
fn test_non_numeric_env_var_falls_back_to_default() {
    let _guard = EnvGuard::set(&[(CHUNK_TIMEOUT_VAR, "abc")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_extraction_timeout_secs, DEFAULT_CHUNK_TIMEOUT_SECS,
        "non-numeric env var must be ignored, default used"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 9 — Max concurrent minimum clamping: zero is not allowed
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=0` is clamped to 1.
/// Zero would deadlock the semaphore-based parallelism control.
#[test]
#[serial]
fn test_max_concurrent_zero_clamped_to_one() {
    let _guard = EnvGuard::set(&[(MAX_CONCURRENT_VAR, "0")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.max_concurrent_extractions, 1,
        "max_concurrent=0 must be clamped to 1 (semaphore cannot have 0 permits)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 10 — Max retries zero is valid (fail-fast mode)
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_CHUNK_MAX_RETRIES=0` is accepted.
/// This enables a "fail fast, no retries" mode for CI or debugging.
#[test]
#[serial]
fn test_max_retries_zero_is_valid() {
    let _guard = EnvGuard::set(&[(MAX_RETRIES_VAR, "0")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_max_retries, 0,
        "max_retries=0 is valid — fail-fast mode with no retries"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 11 — Max retries capped at maximum (safety bound)
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `EDGEQUAKE_CHUNK_MAX_RETRIES=999` is clamped to 20 (MAX_CHUNK_MAX_RETRIES).
#[test]
#[serial]
fn test_max_retries_above_max_is_clamped() {
    let _guard = EnvGuard::set(&[(MAX_RETRIES_VAR, "999")]);

    let config = PipelineConfig::from_env();

    assert_eq!(
        config.chunk_max_retries, MAX_CHUNK_MAX_RETRIES,
        "max_retries=999 must be clamped to MAX_CHUNK_MAX_RETRIES"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 12 — default_pipeline() uses from_env()
// ─────────────────────────────────────────────────────────────────────────────

/// Proves that `Pipeline::default_pipeline()` reflects env vars.
/// This is the integration point — workspace pipelines call `default_pipeline()`.
#[test]
#[serial]
fn test_default_pipeline_respects_env_vars() {
    use edgequake_pipeline::Pipeline;

    let _guard = EnvGuard::set(&[(CHUNK_TIMEOUT_VAR, "300"), (MAX_CONCURRENT_VAR, "8")]);

    let pipeline = Pipeline::default_pipeline();

    assert_eq!(
        pipeline.config().chunk_extraction_timeout_secs,
        300,
        "default_pipeline must honour EDGEQUAKE_CHUNK_TIMEOUT_SECS"
    );
    assert_eq!(
        pipeline.config().max_concurrent_extractions,
        8,
        "default_pipeline must honour EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 13 — SafetyLimitsConfig now accepts up to 3600 s
// ─────────────────────────────────────────────────────────────────────────────

/// Proves that the HTTP-layer safety cap was raised from 600 s to 3600 s.
/// Without this fix, setting `EDGEQUAKE_LLM_TIMEOUT_SECS=1800` would be
/// silently clamped to 600 s, confusing operators who expected 30 minutes.
#[test]
#[serial]
fn test_safety_limits_accepts_timeout_up_to_one_hour() {
    use edgequake_api::safety_limits::{SafetyLimitsConfig, MAXIMUM_TIMEOUT_SECS};

    // Verify the constant was raised
    assert_eq!(
        MAXIMUM_TIMEOUT_SECS, 3600,
        "MAXIMUM_TIMEOUT_SECS must be 3600 (1 hour) for local LLM support"
    );

    // Verify from_env respects 1800 s without clamping
    unsafe { std::env::set_var("EDGEQUAKE_LLM_TIMEOUT_SECS", "1800") };
    let config = SafetyLimitsConfig::from_env();
    unsafe { std::env::remove_var("EDGEQUAKE_LLM_TIMEOUT_SECS") };

    assert_eq!(
        config.timeout.as_secs(),
        1800,
        "EDGEQUAKE_LLM_TIMEOUT_SECS=1800 must not be clamped below 3600 s cap"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 14 — PipelineConfig::default() still uses compile-time constants
// ─────────────────────────────────────────────────────────────────────────────

/// Proves `PipelineConfig::default()` is NOT affected by env vars.
/// This keeps unit tests deterministic regardless of environment.
#[test]
#[serial]
fn test_pipeline_config_default_ignores_env_vars() {
    let _guard = EnvGuard::set(&[(CHUNK_TIMEOUT_VAR, "9999"), (MAX_CONCURRENT_VAR, "99")]);

    let config = PipelineConfig::default();

    assert_eq!(
        config.chunk_extraction_timeout_secs, DEFAULT_CHUNK_TIMEOUT_SECS,
        "PipelineConfig::default() must always return compile-time constants"
    );
    assert_eq!(
        config.max_concurrent_extractions, DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
        "PipelineConfig::default() must always return compile-time constants"
    );
}
