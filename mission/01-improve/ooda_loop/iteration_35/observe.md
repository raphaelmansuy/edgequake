# OODA-35 — Observe

## Target: `crates/edgequake-pipeline/src/error.rs` (391 lines, 0 tests)

### Types with testable pure methods:
1. **ChunkFailure** — struct with 6 fields, no methods
2. **ChunkExtractionOutcome** — enum with 6 methods: is_success, is_failure, chunk_index, as_result, into_result, as_failure
3. **ResilientExtractionResult** — struct with from_outcomes (REDUCE), success_rate, is_complete_success, has_any_success, is_complete_failure, summary
4. **PipelineError** — enum with Display implementations via thiserror

### All methods are pure — no I/O, no async.
