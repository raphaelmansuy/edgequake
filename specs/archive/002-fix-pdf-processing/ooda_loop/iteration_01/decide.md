# Iteration 01 - Decide

## Re-read Mission

**Mission file**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/002-fix-pdf-processing.md`

## Decision: Initialize PDF Progress in Upload Handler

### Priority 1 (Immediate Fix)

**Action**: Add `start_pdf_progress()` call in `upload_pdf_document()` before returning response.

### Implementation

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

**Location**: After line 461 (`create_pdf_processing_task`), before line 468 (response construction)

```rust
// OODA-01: Initialize progress tracking immediately to prevent 404 on poll
// WHY: Frontend polls /pdf/progress/{track_id} immediately after upload
//      but progress was only initialized when task started (race condition)
let effective_track_id = options.track_id.clone().unwrap_or_else(|| task_id.clone());
state
    .pipeline_state
    .start_pdf_progress(&effective_track_id, &pdf_id.to_string(), &filename)
    .await;
```

### Rationale

1. **Root Cause**: Progress is only initialized in `on_extraction_start()` callback
2. **Effect**: Frontend gets 404 when polling immediately after upload
3. **Fix**: Initialize progress before returning upload response
4. **Track ID**: Use `options.track_id` if provided, otherwise fall back to `task_id`

### Risk Mitigation

| Risk                  | Mitigation                                                                                |
| --------------------- | ----------------------------------------------------------------------------------------- |
| Double initialization | `on_extraction_start` will re-initialize, which is idempotent (overwrites with same data) |
| Track ID mismatch     | Use same logic as callback - `options.track_id` or `task_id`                              |
| Memory leak           | Cleanup already exists in `complete_pdf()` and `fail_pdf()`                               |

### Test Plan

1. Upload PDF via Playwright
2. Verify `/pdf/progress/{track_id}` returns 200 immediately
3. Watch processing complete
4. Verify document appears in document list

### Checklist

- [ ] Add progress initialization in upload handler
- [ ] Test with Playwright
- [ ] Verify no 404 on progress poll
- [ ] Verify processing completes successfully
- [ ] Commit changes
