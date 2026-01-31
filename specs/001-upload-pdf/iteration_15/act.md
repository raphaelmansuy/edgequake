# Iteration 15: Act

## Changes Made

### File 1: `edgequake/crates/edgequake-api/src/handlers/websocket.rs`

- Lines: 28 (import Path), 212-380 (new handler)
- Change: Added `ws_progress_by_track_id` handler and `handle_filtered_progress_socket`
- Why: Enables filtered WebSocket streaming for specific PDF uploads

### File 2: `edgequake/crates/edgequake-api/src/routes.rs`

- Lines: 104-107
- Change: Added route `/ws/progress/{track_id}`
- Why: Exposes the new filtered WebSocket endpoint

## Handler Features

1. **Initial Snapshot**: Sends `ProgressSnapshot` with current `PdfUploadProgress` on connect
2. **Filtered Events**: Only forwards `PdfPageProgress` events matching `task_id`
3. **Heartbeat**: 30-second keepalive to prevent connection timeout
4. **Status Command**: Client can send "status" to get current progress
5. **Clean Disconnect**: Logs disconnect events with track_id for debugging

## Key Functions

### `ws_progress_by_track_id`

Handler that upgrades HTTP to WebSocket with `track_id` path parameter.

### `handle_filtered_progress_socket`

Main event loop that:

- Receives broadcast events
- Filters to only matching `task_id`
- Sends to client

### `matches_track_id`

```rust
fn matches_track_id(event: &ProgressEvent, track_id: &str) -> bool {
    match event {
        ProgressEvent::PdfPageProgress { task_id, .. } => task_id == track_id,
        ProgressEvent::ChunkFailure { task_id, .. } => task_id == track_id,
        _ => false,
    }
}
```

## Verification

```bash
# Build
cargo build --package edgequake-api
# Result: Success (5 warnings, unrelated)

# Full test suite
cargo test --package edgequake-api --lib
# Result: 435 passed; 0 failed
```

## Commit

- SHA: `d364e45b`
- Message: "OODA-15: Add filtered WebSocket /ws/progress/{track_id} endpoint"

## WebSocket Endpoints Summary

| Endpoint                      | Purpose             | Filter          |
| ----------------------------- | ------------------- | --------------- |
| `GET /ws/pipeline/progress`   | All pipeline events | None (admin)    |
| `GET /ws/progress/{track_id}` | PDF upload progress | `task_id` match |

## Message Types (Server → Client)

```javascript
// On connect
{"type":"Connected","data":{"message":"Connected to progress stream for pdf-abc123"}}

// Initial snapshot
{"type":"ProgressSnapshot","data":{...PdfUploadProgress...}}

// Progress events (filtered)
{"type":"PdfPageProgress","data":{"pdf_id":"...","task_id":"pdf-abc123","page_num":5,"total_pages":10,...}}

// Heartbeat (every 30s)
{"type":"Heartbeat","data":{"timestamp":"2025-06-01T12:00:00Z"}}
```

## Client Example

```javascript
const ws = new WebSocket(`ws://localhost:8020/ws/progress/${trackId}`);
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  switch (msg.type) {
    case "ProgressSnapshot":
      updateProgressUI(msg.data);
      break;
    case "PdfPageProgress":
      updatePageProgress(msg.data.page_num, msg.data.total_pages);
      break;
  }
};
```

## Next Iteration Focus

OODA-16: Connect processor to call `start_pdf_progress()` with filename

- Currently callbacks use empty filename
- Need to pass actual filename from upload through to callback
- Also consider cleanup: call `remove_pdf_progress()` after completion
