# Iteration 14: Act

## Changes Made

### File 1: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`
- Lines: 50 (import), 712-762 (handler)
- Change: Added `get_pdf_progress` handler function
- Why: Provides HTTP endpoint for querying PDF upload progress

### File 2: `edgequake/crates/edgequake-api/src/routes.rs`
- Lines: 216-219
- Change: Added route `/documents/pdf/progress/{track_id}` before `{pdf_id}` routes
- Why: More specific route must come before parameterized catch-all

## Handler Implementation

```rust
pub async fn get_pdf_progress(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let progress = state
        .pipeline_state
        .get_pdf_progress(&track_id)
        .await
        .ok_or_else(|| ApiError::NotFound(...))?;

    let json_value = serde_json::to_value(&progress)?;
    Ok(Json(json_value))
}
```

## Design Decisions

1. **Use `serde_json::Value` for response**: Avoids adding utoipa dependency to edgequake-tasks
2. **404 for not found**: Clear semantics - either upload not started or already cleaned up
3. **Route placement**: `/progress/{track_id}` before `/{pdf_id}` to avoid route conflict

## Tests Added/Modified

- No new tests added in this iteration (handler is thin wrapper)
- Existing `pipeline_progress_callback` tests verify storage
- 435 total tests pass

## Verification

```bash
# Build
cargo build --package edgequake-api
# Result: Success (5 warnings, unrelated)

# Full test suite
cargo test --package edgequake-api --lib
# Result: 435 passed; 0 failed
```

## Evidence

```
test result: ok. 435 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.85s
```

## Commit

- SHA: `c77f96ab`
- Message: "OODA-14: Implement GET /documents/pdf/progress/{track_id} endpoint"

## API Documentation

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/v1/documents/pdf/progress/{track_id}` | `get_pdf_progress` | Get upload progress |

### Response Example (200 OK)

```json
{
  "track_id": "pdf-abc123",
  "pdf_id": "uuid-here",
  "filename": "document.pdf",
  "phases": [
    {
      "phase": "upload",
      "status": "complete",
      "current": 1,
      "total": 1,
      "percentage": 100.0
    },
    {
      "phase": "pdf_conversion",
      "status": "active",
      "current": 5,
      "total": 10,
      "percentage": 50.0,
      "eta_seconds": 15,
      "message": "Extracted page 5 of 10"
    }
    // ... 4 more phases
  ],
  "overall_percentage": 25.0,
  "eta_seconds": 45,
  "is_complete": false,
  "is_failed": false
}
```

### Response (404 Not Found)

```json
{
  "error": "Progress not found. Upload may have completed or not yet started."
}
```

## Next Iteration Focus

OODA-15: Implement WebSocket `/ws/progress/{track_id}` endpoint
- Real-time progress updates via WebSocket
- Filtering by track_id for specific upload
- Heartbeat/ping mechanism
- Reconnection support on frontend
