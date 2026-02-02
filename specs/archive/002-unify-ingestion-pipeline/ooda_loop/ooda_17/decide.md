# OODA-17: Decide - Action Plan

## Date: 2026-02-01

## Decision

Implement **Option A: Add Phase Tracking in processor.rs**

## Specific Changes

### 1. File: `edgequake-api/src/processor.rs`

#### 1.1 Add Import

```rust
use edgequake_tasks::{PipelinePhase, PipelineState, ...};
```

#### 1.2 Add PDF Source Detection

After document status is set to "chunking", add:

```rust
let is_pdf_source = source_type == "pdf";
let track_id = task.track_id.clone();
```

#### 1.3 Add Phase Tracking Calls

| Location          | Phase Start                     | Phase Complete                     |
| ----------------- | ------------------------------- | ---------------------------------- |
| Before chunking   | `start_pdf_phase(Chunking)`     | -                                  |
| After chunking    | -                               | `complete_pdf_phase(Chunking)`     |
| Before extraction | `start_pdf_phase(Extraction)`   | -                                  |
| After extraction  | -                               | `complete_pdf_phase(Extraction)`   |
| Before embedding  | `start_pdf_phase(Embedding)`    | -                                  |
| After embedding   | -                               | `complete_pdf_phase(Embedding)`    |
| Before storage    | `start_pdf_phase(GraphStorage)` | -                                  |
| After storage     | -                               | `complete_pdf_phase(GraphStorage)` |

### 2. Conditional Logic

All phase tracking calls wrapped in:

```rust
if is_pdf_source {
    self.pipeline_state.start_pdf_phase(&track_id, PipelinePhase::Chunking, ...).await;
}
```

This ensures:

- Markdown processing is unaffected
- Only PDFs get granular phase tracking
- Existing behavior preserved

## Priority

1. **High**: Add phase tracking for PDF uploads
2. **Medium**: Verify with E2E testing
3. **Low**: Add unit tests for phase transitions

## Success Criteria

| Criteria                    | Verification Method |
| --------------------------- | ------------------- |
| PDF shows phase progress    | Playwright E2E test |
| Phases transition correctly | Backend logs        |
| Markdown still works        | Regression test     |
| No performance impact       | Timing logs         |
| All unit tests pass         | `cargo test`        |

## Rollback Plan

If issues arise:

1. Revert the 4 code changes in processor.rs
2. Original functionality restored

## Files to Modify

| File                             | Change Type                    |
| -------------------------------- | ------------------------------ |
| `edgequake-api/src/processor.rs` | Add conditional phase tracking |

## No Changes Required

| File                            | Reason                                 |
| ------------------------------- | -------------------------------------- |
| `pipeline_progress_callback.rs` | Already tracks PdfConversion correctly |
| `pipeline_state.rs`             | Methods already exist                  |
| `progress.rs`                   | Types already defined                  |
| Frontend components             | Already render phase progress          |
