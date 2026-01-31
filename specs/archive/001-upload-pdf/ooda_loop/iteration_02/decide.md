# Iteration 02: Decide

## Mission Re-Read ✅

- [x] Re-read mission file at `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed phase: Architecture & Design (Iterations 1-10)

## Decision

We will implement the **ProgressCallback trait and NoopProgress** in `edgequake-pdf`:

1. Create `progress.rs` module with ProgressCallback trait
2. Add NoopProgress default implementation
3. Export from lib.rs
4. Add unit tests

This iteration focuses on the trait definition only. Integration into PdfExtractor comes next iteration.

## Rationale (First Principles)

1. **Trait before implementation**: Define the contract first
2. **Default no-op**: Existing code doesn't break
3. **Simple interface**: 6 methods covering all extraction events
4. **Send + Sync**: Safe for async/parallel use
5. **Arc<dyn>**: Dynamic dispatch for flexibility

## Action Items

1. [x] Create `edgequake/crates/edgequake-pdf/src/progress.rs` - Est: 10 min
   - Define `ProgressCallback` trait with 6 methods
   - Implement `NoopProgress` struct
   - Add `LoggingProgress` for debugging
   - Add documentation with WHY comments

2. [x] Update `edgequake/crates/edgequake-pdf/src/lib.rs` - Est: 2 min
   - Add `pub mod progress;`
   - Re-export `ProgressCallback`, `NoopProgress`, `LoggingProgress`

3. [x] Add unit tests - Est: 5 min
   - Test NoopProgress doesn't panic
   - Test LoggingProgress records calls

## Success Metrics

- [x] `cargo build --package edgequake-pdf` succeeds
- [x] `cargo test --package edgequake-pdf` passes
- [x] New types are exported and usable
- [x] Documentation builds cleanly

## Testing Strategy

- **Unit tests**: In `edgequake-pdf/src/progress.rs`
  - `test_noop_progress_does_nothing()`
  - `test_logging_progress_records_calls()`
- **Integration tests**: None yet (trait only)

## Commit Message

```
OODA-02: Add ProgressCallback trait for PDF extraction progress

- Created ProgressCallback trait with 6 lifecycle methods
- Added NoopProgress for backward compatibility
- Added LoggingProgress for debugging
- Methods: on_extraction_start, on_page_start, on_page_complete,
           on_page_error, on_extraction_complete, on_progress

Why: Enables page-by-page progress reporting during PDF extraction
Tests: cargo test --package edgequake-pdf progress
```
