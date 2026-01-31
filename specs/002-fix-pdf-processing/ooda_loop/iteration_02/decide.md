# Iteration 02 - Decide

## Re-read Mission

**Mission file**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/002-fix-pdf-processing.md`

## Decision: Add Progress Initialization to Duplicate Path

### Implementation

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

**Location**: Lines 411-427, right after "Duplicate PDF upload detected" warning, before `return`

```rust
// OODA-01 FIX: Initialize progress even for duplicates
//
// WHY: Frontend polls /pdf/progress/{track_id} immediately after upload.
//      Even for duplicates, we need to return a valid progress entry
//      so the frontend doesn't get a 404 error.
//
// The duplicate response tells the frontend it's already processed,
// but the progress entry needs to exist for the initial poll.
if let Some(ref track_id) = options.track_id {
    info!(
        "OODA-01: Initializing PDF progress for duplicate, track_id={}, pdf_id={}, filename={}",
        track_id, existing.pdf_id, existing.filename
    );
    state
        .pipeline_state
        .start_pdf_progress(track_id, &existing.pdf_id.to_string(), &existing.filename)
        .await;
}
```

### Rationale

1. **Symmetry**: Both code paths now initialize progress before returning
2. **Defensive**: Progress exists for any track_id frontend generates
3. **Idempotent**: Calling `start_pdf_progress` multiple times is safe
4. **Minimal Change**: Only adds initialization, doesn't change response

### Alternative Considered

Could have also marked progress as "completed" immediately since duplicate means already processed. However, this would require adding a new method `complete_pdf_progress()` which doesn't exist. The simpler approach is to just initialize the progress entry, which is enough to prevent 404.

### Test Plan

1. Upload same PDF file again (will be duplicate)
2. Verify `/pdf/progress/{track_id}` returns 200 OK
3. Verify upload response shows status "duplicate"
4. Verify backend log shows "OODA-01: Initializing PDF progress for duplicate"
