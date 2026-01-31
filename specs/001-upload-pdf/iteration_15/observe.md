# Iteration 15: Observe

## Mission Re-Read ✅
- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: WebSocket with < 500ms latency, track_id filtering
- [x] Current phase: Phase 2 - Backend Implementation (Iterations 11-25)
- [x] This iteration: Add WebSocket `/ws/progress/{track_id}` endpoint

## Code Analysis

### Existing WebSocket Infrastructure
- File: `handlers/websocket.rs`
- Handler: `ws_pipeline_progress`
- Route: `/ws/pipeline/progress`
- Behavior: Broadcasts ALL ProgressEvents to ALL connected clients
- Uses: `state.progress_broadcaster.subscribe()` for events

### ProgressEvent Structure (websocket_types.rs)
```rust
pub enum ProgressEvent {
    PdfPageProgress {
        pdf_id: String,
        task_id: String,  // <-- This is our filter key!
        page_num: u32,
        total_pages: u32,
        phase: String,
        ...
    },
    // Other events...
}
```

### Route Registration (routes.rs)
```rust
.route("/ws/pipeline/progress", get(handlers::ws_pipeline_progress))
```

## Data Gathered

1. `task_id` in `PdfPageProgress` matches `track_id` (same value)
2. Existing handler broadcasts to all clients (no filtering)
3. We need a NEW endpoint that filters by `track_id`
4. Filter logic: skip events where `task_id != track_id`

## Questions to Answer

1. Should we modify existing handler or add new one? → ADD NEW (less risk)
2. Where to add the new handler? → Same file, `websocket.rs`
3. How to extract `track_id` from path with WebSocket? → `Path<String>` extractor
4. How to filter events? → Match on `task_id` field in event

## Implementation Plan

1. Add new handler `ws_progress_by_track_id` with `Path(track_id): Path<String>`
2. Filter incoming broadcast events to only include matching `task_id`
3. Register route at `/ws/progress/{track_id}`
4. Keep existing `/ws/pipeline/progress` for admin/monitoring use
