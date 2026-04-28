# SPEC-010-T: Configurable Timeout & Concurrency for Chunk Entity Extraction

> **Issue**: [#194 — Timeout failures on large documents](https://github.com/raphaelmansuy/edgequake/issues/194)
> **Status**: IN PROGRESS (v0.11.3)
> **Author**: EdgeQuake Engineering
> **Date**: 2026-04-29

---

## 1. WHY — The Problem First

### 1.1 Symptom

Users with **local LLM providers** (Ollama, LM Studio, local Mistral) running on
consumer hardware report consistent ingestion failures on large documents:

```
Pipeline processing failed: Entity extraction error:
  All 1 chunks failed extraction.
  Failures: Chunk 0: Timeout after 180s (attempt 3/3) - 184/0
```

### 1.2 Root Cause Tree

```
                   [INGESTION FAILS ON LARGE DOCS]
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
    [RC-1: Hard-coded   [RC-2: Hard-coded   [RC-3: No concurrency
     180s per-chunk      3 max retries       knob — 16 concurrent
     timeout]            not configurable]   requests hammer slow
          │                                   Ollama instances]
          │
          ▼
  default_chunk_timeout() returns 180
  PipelineConfig::default() uses it
  NO env var reading in PipelineConfig
          │
          ▼
  User sets EDGEQUAKE_LLM_TIMEOUT_SECS=600
  → Only affects HTTP-layer SafetyLimitsConfig
  → Does NOT affect per-chunk pipeline timeout ← DISCONNECT
```

### 1.3 Two Different Timeout Layers (Confusion)

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 1 — HTTP / Provider Safety (edgequake-api)               │
│                                                                  │
│  SafetyLimitsConfig::from_env()                                  │
│    reads: EDGEQUAKE_LLM_TIMEOUT_SECS  (default 600s)            │
│    clamps: 10s .. 600s                                           │
│    controls: tokio::time::timeout around HTTP call               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼ ONE LLM HTTP call per chunk
┌─────────────────────────────────────────────────────────────────┐
│  Layer 2 — Per-Chunk Pipeline Timeout (edgequake-pipeline)      │
│                                                                  │
│  PipelineConfig { chunk_extraction_timeout_secs: 180 }          │
│    reads: *** NOTHING FROM ENV ***   ← THE BUG                  │
│    controls: tokio::time::timeout around extractor.extract()    │
│    result: "Timeout after 180s (attempt 3/3)"                   │
└─────────────────────────────────────────────────────────────────┘
```

**Key insight**: The 600s safety-layer timeout is IRRELEVANT for the error users
see, because the pipeline-layer 180s timeout fires first, 3 times, then gives up.

### 1.4 The Cascade Effect

```
Document with 142 chunks submitted
        │
        ▼
 max_concurrent_extractions = 16 (no env var)
        │
        ▼
 16 parallel Ollama requests on slow hardware
        │
        ▼
 Ollama queue backs up → all 16 exceed 180s
        │
        ▼
 Retry: 16 MORE requests → queue backs up more
        │
        ▼
 Retry: 16 MORE requests → total 3 × 16 = 48 timed-out requests
        │
        ▼
 "All 1 chunks failed" for EVERY chunk ← catastrophic failure
```

### 1.5 Time Math (Why 180s × 3 Retries Is Catastrophic)

For a single chunk failing all 3 attempts:
```
Attempt 1: 180s timeout
  Wait:    1s (initial_delay × 2^0)
Attempt 2: 180s timeout
  Wait:    2s (initial_delay × 2^1)
Attempt 3: 180s timeout
─────────────────────────
Total:    543s ≈ 9 minutes per FAILED chunk
× 142 chunks (sequentially after semaphore) = unacceptable
```

---

## 2. WHAT — Requirements

### Functional Requirements

| ID     | Requirement                                                                                 |
| ------ | ------------------------------------------------------------------------------------------- |
| FR-T01 | `chunk_extraction_timeout_secs` MUST be overridable via `EDGEQUAKE_CHUNK_TIMEOUT_SECS`      |
| FR-T02 | `chunk_max_retries` MUST be overridable via `EDGEQUAKE_CHUNK_MAX_RETRIES`                   |
| FR-T03 | `initial_retry_delay_ms` MUST be overridable via `EDGEQUAKE_CHUNK_RETRY_DELAY_MS`           |
| FR-T04 | `max_concurrent_extractions` MUST be overridable via `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` |
| FR-T05 | All env vars MUST fall back to current defaults (no breaking change)                        |
| FR-T06 | Invalid values (non-numeric, out of range) MUST be silently ignored, defaults used          |
| FR-T07 | Safety bounds MUST be enforced: timeout ≥ 10s, retries ≥ 0, concurrency ≥ 1                 |

### Non-Functional Requirements

| ID      | Requirement                                                                     |
| ------- | ------------------------------------------------------------------------------- |
| NFR-T01 | No performance regression for users NOT using env vars                          |
| NFR-T02 | Reading env vars MUST happen at pipeline construction, NOT per-chunk extraction |
| NFR-T03 | Startup log MUST show effective timeout configuration                           |

---

## 3. HOW — Solution Design

### 3.1 Add `PipelineConfig::from_env()` (DRY, mirrors SafetyLimitsConfig pattern)

```
edgequake-pipeline/src/pipeline/mod.rs

impl PipelineConfig {
    /// Create config from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let chunk_timeout = read_env_u64("EDGEQUAKE_CHUNK_TIMEOUT_SECS", 180, 10, u64::MAX);
        let max_retries   = read_env_u32("EDGEQUAKE_CHUNK_MAX_RETRIES",   3,   0, 20);
        let retry_delay   = read_env_u64("EDGEQUAKE_CHUNK_RETRY_DELAY_MS",1000,0, 60_000);
        let max_concurrent= read_env_usize("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS", 16, 1, 256);

        Self {
            chunk_extraction_timeout_secs: chunk_timeout,
            chunk_max_retries: max_retries,
            initial_retry_delay_ms: retry_delay,
            max_concurrent_extractions: max_concurrent,
            ..Self::default()
        }
    }
}
```

### 3.2 Change `Pipeline::default_pipeline()` to use `from_env()`

```
pub fn default_pipeline() -> Self {
    Self::new(PipelineConfig::from_env())  // was: PipelineConfig::default()
}
```

### 3.3 Remove the 600s cap on `MAXIMUM_TIMEOUT_SECS`

The SafetyLimitsConfig hard-caps at 600s. For local LLMs this is too strict.
The cap should be raised or made provider-aware:

```
pub const MAXIMUM_TIMEOUT_SECS: u64 = 3600; // 1 hour for local LLMs
```

**OR** (preferred) — honour the env var without an upper cap:
The per-chunk pipeline timeout is the real safeguard for ingestion; the safety
wrapper is a secondary backstop. If a user sets `EDGEQUAKE_CHUNK_TIMEOUT_SECS=600`
and the safety layer caps at 600, the safety layer timeout fires first (same value)
which is fine. But if user sets `EDGEQUAKE_CHUNK_TIMEOUT_SECS=1200`, the safety
layer fires at 600 and the user sees confusing errors.

**Decision**: Raise `MAXIMUM_TIMEOUT_SECS` to 3600 and add a separate `EDGEQUAKE_LLM_TIMEOUT_SECS` cap raise.

---

## 4. Environment Variable Reference

After this fix, the following env vars control ingestion performance:

| Variable                               | Default | Min | Max   | Description                             |
| -------------------------------------- | ------- | --- | ----- | --------------------------------------- |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS`         | 180     | 10  | ∞     | Timeout per chunk LLM call              |
| `EDGEQUAKE_CHUNK_MAX_RETRIES`          | 3       | 0   | 20    | Max retry attempts per chunk            |
| `EDGEQUAKE_CHUNK_RETRY_DELAY_MS`       | 1000    | 0   | 60000 | Initial backoff delay in ms             |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | 16      | 1   | 256   | Max parallel LLM calls per document     |
| `EDGEQUAKE_LLM_TIMEOUT_SECS`           | 600     | 10  | 3600  | HTTP safety-layer timeout (per request) |

### Recommended `.env` for slow local Ollama:

```env
# Slow GPU / shared Ollama instance
EDGEQUAKE_CHUNK_TIMEOUT_SECS=600
EDGEQUAKE_CHUNK_MAX_RETRIES=2
EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4
EDGEQUAKE_LLM_TIMEOUT_SECS=600
```

### Recommended `.env` for fast cloud API (OpenAI):

```env
# Fast cloud — default values work, or tighten for cost control
EDGEQUAKE_CHUNK_TIMEOUT_SECS=60
EDGEQUAKE_CHUNK_MAX_RETRIES=3
EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=16
```

---

## 5. Test Strategy

### Unit Tests (no postgres)

1. `test_pipeline_config_from_env_reads_chunk_timeout` — set env var, verify value
2. `test_pipeline_config_from_env_fallback_to_default` — unset env var, verify default
3. `test_pipeline_config_from_env_clamps_below_minimum` — set `=5`, verify clamped to 10
4. `test_pipeline_config_from_env_ignores_invalid_value` — set `=abc`, verify default
5. `test_pipeline_config_all_env_vars` — set all 4, verify all read correctly

### Integration / E2E Tests

6. `test_timeout_env_var_propagates_to_pipeline` — construct pipeline, verify config matches env
7. `test_low_concurrency_env_var_reduces_parallel_load` — verify semaphore is bounded

---

## 6. Breaking Changes

**None.** All changes are additive:
- `PipelineConfig::default()` still returns the same hardcoded values (used by tests)
- `Pipeline::default_pipeline()` now calls `from_env()` which falls back to defaults
- All env vars default to current hardcoded values when unset

---

## 7. Cross-References

- Root Cause Analysis: [root_cause_analysis.md](./root_cause_analysis.md)
- Fix Specification: [fix_specification.md](./fix_specification.md)
- Mitigation Strategies: [mitigation_strategies.md](./mitigation_strategies.md)
- GitHub Issue: https://github.com/raphaelmansuy/edgequake/issues/194
