# Iteration 14: Orient

## Gap Analysis

| Current State | Desired State | Gap | Priority |
|--------------|---------------|-----|----------|
| Progress stored by `track_id` | Need queryable endpoint | No GET endpoint exists | HIGH |
| `PipelineState.get_pdf_progress(track_id)` exists | HTTP API wrapper | Just need handler + route | HIGH |
| Routes exist for `/documents/pdf/{pdf_id}` | Need `/documents/pdf/progress/{track_id}` | Add new route | HIGH |

## Risk Assessment

- **Risk 1**: Using `track_id` vs `pdf_id` in URL
  - Mitigation: Use `track_id` since that's the storage key
  - Alternative: Could add bidirectional lookup later

- **Risk 2**: 404 if progress doesn't exist (completed or not started)
  - Mitigation: Return 404 with helpful message

- **Risk 3**: Progress data may be stale (fire-and-forget spawns)
  - Mitigation: Client should poll or use WebSocket; single GET is snapshot

## First Principles Analysis

- **Core problem**: Frontend needs to query progress for a specific upload
- **Fundamental constraint**: Progress is keyed by `track_id` in HashMap
- **Minimal solution**: Handler that calls `get_pdf_progress(track_id)` and returns JSON
- **Why this matters**: Enables frontend to poll progress, foundation for WebSocket

## Alternative Approaches

1. **Option A: GET /documents/pdf/progress/{track_id}**
   - Pros: Direct key lookup, O(1), matches storage model
   - Cons: Client must track `track_id` (already does for batch uploads)
   - Selected: YES

2. **Option B: GET /documents/pdf/{pdf_id}/progress**
   - Pros: Matches REST convention for sub-resources
   - Cons: Requires reverse lookup from pdf_id → track_id (not currently stored)
   - Selected: NO (requires additional storage)

3. **Option C: Both endpoints**
   - Pros: Maximum flexibility
   - Cons: More code, maintenance burden
   - Selected: FUTURE (can add Option B later with reverse index)

## Response Structure

The endpoint will return `PdfUploadProgress` which already has:
```rust
pub struct PdfUploadProgress {
    pub track_id: String,
    pub pdf_id: String,
    pub document_id: Option<String>,
    pub filename: String,
    pub phases: Vec<PhaseProgress>,
    pub overall_percentage: f32,
    pub eta_seconds: Option<u64>,
    pub is_complete: bool,
    // ... more fields
}
```

This is already `Serialize`, can return directly as JSON.

## Route Placement

Must add BEFORE the `/documents/{document_id}` catch-all route:
```rust
.route("/documents/pdf/progress/{track_id}", get(handlers::get_pdf_progress))
```
