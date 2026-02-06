# OODA Iteration 03: Decide - Solution Design

## Chosen Approach

**Direct Fix**: Modify `processor.rs` to store `track_id` in document metadata.

## Implementation Plan

### Step 1: Add track_id to PDF Metadata JSON

**Location**: `processor.rs` line ~1700 (inside `process_text_insert`)

**Before**:
```rust
let metadata_json = json!({
    "id": early_doc_id,
    "source_type": "pdf_processor",
    // ... other fields
});
```

**After**:
```rust
let metadata_json = json!({
    "id": early_doc_id,
    "source_type": "pdf_processor",
    "track_id": task.track_id.clone(), // ADDED
    // ... other fields
});
```

### Step 2: Update ensure_document_source_type Function Signature

**Location**: `processor.rs` line ~1384

**Before**:
```rust
async fn ensure_document_source_type(
    ...
    content_summary: &str,
    context: &str,
) -> Result<(), EdgeQuakeError>
```

**After**:
```rust
async fn ensure_document_source_type(
    ...
    content_summary: &str,
    context: &str,
    track_id: Option<&str>,  // ADDED
) -> Result<(), EdgeQuakeError>
```

### Step 3: Store track_id in Metadata Inside Function

**Location**: Inside `ensure_document_source_type`, when creating/updating metadata JSON

Add `"track_id": track_id` to the metadata JSON object.

### Step 4: Update Call Site

**Location**: Where `ensure_document_source_type` is called from

Pass `Some(&task.track_id)` as the new parameter.

## Alternative Approaches Considered

1. **Store track_id in separate table**: More complex, requires schema change
2. **Modify DocumentSummary to always include track_id from tasks table**: Would require JOIN query, more expensive
3. **Client-side tracking**: Unreliable, state lost on refresh

## Decision Rationale

- Minimal code change (3 locations)
- No database schema change needed
- Backward compatible with existing documents
- Follows existing pattern of storing task metadata in JSON field
- Quick to implement and test
