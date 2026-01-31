# Iteration 15: Decide

## Decision

Implement `GET /ws/progress/{track_id}` WebSocket endpoint that filters events to only send `PdfPageProgress` events matching the specified `track_id`.

## Rationale

1. **Clean API**: Client connects to their specific upload's stream
2. **Reduced bandwidth**: Only relevant events sent over wire
3. **Simpler frontend**: No filtering logic needed in JavaScript
4. **Coexistence**: Keeps existing `/ws/pipeline/progress` for admin use

## Action Items

1. [x] Add `ws_progress_by_track_id` handler in `websocket.rs`
2. [x] Add route `/ws/progress/{track_id}` in `routes.rs`
3. [x] Filter broadcast events by matching `task_id`
4. [x] Send initial progress snapshot from `get_pdf_progress()`
5. [x] Keep heartbeat mechanism for connection keepalive

## Success Metrics

- [x] Handler compiles without errors
- [x] Route registered correctly
- [x] Only `PdfPageProgress` events with matching `task_id` are sent
- [x] Initial snapshot sent on connection

## Handler Implementation

```rust
pub async fn ws_progress_by_track_id(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_filtered_progress_socket(socket, state, track_id))
}

async fn handle_filtered_progress_socket(socket: WebSocket, state: AppState, track_id: String) {
    // ... setup similar to existing handler ...
    
    // Filter loop
    loop {
        tokio::select! {
            result = progress_rx.recv() => {
                match result {
                    Ok(event) => {
                        // Only forward PdfPageProgress with matching task_id
                        if matches_track_id(&event, &track_id) {
                            send_event(&mut sender, &event).await?;
                        }
                    }
                    // ...
                }
            }
            // ... heartbeat ...
        }
    }
}

fn matches_track_id(event: &ProgressEvent, track_id: &str) -> bool {
    match event {
        ProgressEvent::PdfPageProgress { task_id, .. } => task_id == track_id,
        _ => false,
    }
}
```
