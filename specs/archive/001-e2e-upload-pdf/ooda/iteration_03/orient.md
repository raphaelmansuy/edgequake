# OODA Iteration 03: Orient - Root Cause Analysis

## Problem Domain

The cancel functionality consists of three layers:

1. **Frontend UI** - Cancel button in dropdown menu (works correctly)
2. **Frontend API** - `cancelTask(trackId)` function calls `POST /api/v1/tasks/{trackId}/cancel` (works correctly)
3. **Backend Storage** - Document metadata must contain `track_id` for cancel to work (**BROKEN**)

## Code Flow Analysis

### Document Creation Flow

```
1. User uploads PDF
   ↓
2. Frontend calls POST /api/v1/documents/upload_pdf
   ↓
3. Backend creates processing task with track_id (e.g., "task_1738837200_abc123")
   ↓
4. Backend calls ensure_document_source_type() to create/update document metadata
   ↓
5. [BUG] track_id is NOT passed to ensure_document_source_type()
   ↓
6. Document metadata saved WITHOUT track_id
   ↓
7. API returns document with track_id = null
   ↓
8. Frontend cancel button condition fails: doc.track_id && ... = false
   ↓
9. Cancel button NOT rendered
```

### Location of Bug

**File**: `edgequake/crates/edgequake-api/src/processor.rs`

**Function 1**: `process_text_insert()` (lines 650-658)

- Creates PDF processing metadata JSON
- Does NOT include `track_id` in the JSON

**Function 2**: `ensure_document_source_type()` (lines 1384-1510)

- Creates/updates document metadata
- Does NOT accept `track_id` parameter
- Does NOT store `track_id` in metadata

### Why This Wasn't Caught Earlier

1. Cancel functionality was a secondary feature - primary focus was on PDF extraction
2. E2E tests didn't include cancel flow testing
3. Manual testing likely used different code paths (text documents vs PDF)
4. The track_id WAS being generated and used for task tracking - just not stored in document metadata

## Impact Assessment

- **Severity**: Medium - Cancel button doesn't appear for processing documents
- **User Impact**: Users cannot cancel long-running PDF extractions
- **Scope**: All PDF documents created after initial implementation

## Fix Strategy

1. Add `track_id` parameter to `ensure_document_source_type()` function
2. Store `track_id` in metadata JSON
3. Pass `task.track_id` when calling `ensure_document_source_type()`
4. Rebuild backend
5. Test with fresh document upload

## Risk Analysis

- **Low Risk**: Change is additive - adds new field to metadata
- **Backward Compatible**: Existing documents without track_id will still work (cancel button just won't appear)
- **No Breaking Changes**: API contract unchanged
