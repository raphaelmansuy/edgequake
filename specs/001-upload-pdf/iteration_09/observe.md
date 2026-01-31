# OODA-09: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6-phase progress, edgequake-pdf first, real-time WebSocket
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)

## Code Analysis

### File: `edgequake/crates/edgequake-api/src/processor.rs`

- Lines: 1230-1350 (process_pdf_processing method)
- Purpose: Process PDF tasks through extraction and pipeline
- Current behavior: Calls `extract_to_markdown()` without progress callback
- Dependencies: `PdfExtractor`, `VisionExtractor`, `self.pipeline_state`

### Key Code Locations

1. **Standard text extraction** (lines ~1346):

   ```rust
   let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
   let md = extractor.extract_to_markdown(&pdf.pdf_data).await
   ```

2. **Vision fallback** (lines ~1322):

   ```rust
   let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
   let md = extractor.extract_to_markdown(&pdf.pdf_data).await
   ```

3. **No-vision path** (lines ~1340):
   ```rust
   let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
   let md = extractor.extract_to_markdown(&pdf.pdf_data).await
   ```

## Data Gathered

1. **Available in scope**:
   - `self.pipeline_state` - PipelineState for emitting events
   - `data.pdf_id` - PDF document ID
   - `task.track_id` - Task tracking ID

2. **Required change**:
   - Replace `extract_to_markdown()` with `extract_to_markdown_with_progress()`
   - Create `Arc<PipelineProgressCallback>` with ids

3. **Three code paths** need updating:
   - Vision fallback path
   - No-vision feature path
   - Standard text extraction path

## Questions to Answer Next Iteration

- Should we also wire progress into VisionExtractor?
- Need to import `PipelineProgressCallback` and `ProgressCallback` trait
