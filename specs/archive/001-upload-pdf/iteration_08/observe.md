# OODA-08: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6-phase progress, edgequake-pdf first, real-time WebSocket
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)

## Code Analysis

### File: `edgequake/crates/edgequake-pdf/src/progress.rs`

- `ProgressCallback` trait with methods:
  - `on_phase_start(phase, total_items)`
  - `on_progress(current, total, item_name)`
  - `on_phase_complete(phase, duration, success, error)`
- Implementations: `NoopProgress`, `LoggingProgress`, `CountingProgress`

### File: `edgequake/crates/edgequake-tasks/src/pipeline_state.rs`

- `PipelineState` with `emit_pdf_page_progress()` method (OODA-07)
- Thread-safe via `Arc<RwLock<_>>` and broadcast channel

### File: `edgequake/crates/edgequake-api/src/processor.rs`

- `DocumentTaskProcessor` has `pipeline_state: PipelineState` field
- `process_pdf_processing()` method at lines 1214-1400
- Currently calls `extract_to_markdown()` without progress callback

## Data Gathered

1. **Trait signature** (from edgequake-pdf/progress.rs):

   ```rust
   pub trait ProgressCallback: Send + Sync {
       fn on_phase_start(&self, phase: &str, total_items: usize);
       fn on_progress(&self, current: usize, total: usize, item_name: &str);
       fn on_phase_complete(&self, phase: &str, duration: Duration, success: bool, error: Option<&str>);
   }
   ```

2. **PipelineState method** (from edgequake-tasks):

   ```rust
   pub fn emit_pdf_page_progress(
       &self, pdf_id: String, task_id: String, page_num: u32,
       total_pages: u32, phase: String, markdown_len: usize,
       success: bool, error: Option<String>,
   )
   ```

3. **Integration point**: `process_pdf_processing()` has access to:
   - `self.pipeline_state`
   - `data.pdf_id`
   - `task.track_id`

## Questions to Answer Next Iteration

- Should adapter be a struct or a closure-based callback?
- How to get markdown_len at each page (need to track cumulative)?
