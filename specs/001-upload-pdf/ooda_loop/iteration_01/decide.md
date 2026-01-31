# Iteration 01: Decide

## Mission Re-Read ✅

- [x] Re-read mission file at `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed phase: Architecture & Design (Iterations 1-10)

## Decision

We will implement the **foundation types for 6-phase progress tracking** in this iteration:

1. Add `PipelinePhase` enum with 6 phases to `edgequake-tasks`
2. Add `PhaseProgress` struct for tracking phase state
3. Add `PdfUploadProgress` struct for overall tracking
4. Keep backward compatibility with existing `ChunkProgress`

This is the minimal first step - just the data types, no behavior yet.

## Rationale (First Principles)

1. **Types before behavior**: Define data structures first, then implement
2. **Single Responsibility**: Keep progress types in `edgequake-tasks` where `TaskProgress` already lives
3. **Additive change**: Don't modify existing types, add new ones alongside
4. **No breaking changes**: Existing code continues to work unchanged
5. **Testable units**: Can write unit tests for progress calculations

## Action Items

1. [x] Create `edgequake/crates/edgequake-tasks/src/progress.rs` - Est: 15 min
   - Define `PipelinePhase` enum (6 phases)
   - Define `PhaseProgress` struct
   - Define `PdfUploadProgress` struct
   - Add helper methods for progress calculations

2. [x] Update `edgequake/crates/edgequake-tasks/src/lib.rs` - Est: 2 min
   - Export new progress module

3. [x] Add unit tests for progress types - Est: 10 min
   - Test phase ordering
   - Test progress percentage calculations
   - Test ETA calculations

## Success Metrics

- [x] `cargo build --package edgequake-tasks` succeeds
- [x] `cargo test --package edgequake-tasks` passes
- [x] New types are exported and usable from other crates
- [x] No changes to existing types (backward compatible)

## Testing Strategy

- **Unit tests**: Test progress calculations in `edgequake-tasks/src/progress.rs`
  - `test_pipeline_phase_ordering()`
  - `test_phase_progress_percentage()`
  - `test_overall_progress_calculation()`
  - `test_eta_calculation()`
- **Integration tests**: None needed for pure data types
- **Manual verification**: `cargo doc --package edgequake-tasks --open` to verify docs

## File Changes Summary

| File                              | Change Type | Description                            |
| --------------------------------- | ----------- | -------------------------------------- |
| `edgequake-tasks/src/progress.rs` | CREATE      | New module with phase tracking types   |
| `edgequake-tasks/src/lib.rs`      | MODIFY      | Add `pub mod progress;` and re-exports |

## Commit Message

```
OODA-01: Add progress tracking types for 6-phase PDF pipeline

- Created PipelinePhase enum: Upload, PdfConversion, Chunking, Embedding, Extraction, GraphStorage
- Created PhaseProgress struct with current/total/percentage/eta
- Created PdfUploadProgress struct for overall tracking
- Added helper methods for progress calculations
- Added unit tests for all new types

Why: Foundation for multi-phase progress monitoring (SPEC-001-upload-pdf)
Tests: cargo test --package edgequake-tasks progress
```
