# Iteration 15: Orient

## Gap Analysis

| Current State | Desired State | Gap | Priority |
|--------------|---------------|-----|----------|
| `/ws/pipeline/progress` broadcasts ALL events | Need filtered `/ws/progress/{track_id}` | New filtered handler | HIGH |
| Client receives all pipeline events | Client receives only its upload's events | Filter on `task_id` | HIGH |

## Risk Assessment

- **Risk 1**: Many simultaneous WebSocket connections per upload
  - Mitigation: Connection limit per IP, heartbeat timeout cleanup
  - Note: For MVP, this is acceptable

- **Risk 2**: Filtering overhead
  - Mitigation: Simple string comparison is O(1)
  - Filtering happens client-side of broadcast, minimal overhead

## First Principles Analysis

- **Core problem**: Frontend connecting to global stream gets ALL events, must filter client-side
- **Fundamental constraint**: Broadcast channel sends to ALL subscribers
- **Minimal solution**: Server-side filter before sending to WebSocket
- **Why this matters**: Less bandwidth, simpler frontend code, cleaner separation

## Alternative Approaches

1. **Option A: New filtered handler (server-side filter)**
   - Pros: Clean API, less frontend work, efficient
   - Cons: More server-side code
   - Selected: YES

2. **Option B: Client-side filter (frontend ignores irrelevant events)**
   - Pros: No backend changes
   - Cons: Wastes bandwidth, complex frontend, not scalable
   - Selected: NO

3. **Option C: Separate broadcast channels per track_id**
   - Pros: Most efficient for high-scale
   - Cons: Complex channel management, memory overhead
   - Selected: FUTURE (if scaling issues arise)

## Filter Logic

When receiving broadcast event, check if it matches track_id:

```rust
match &event {
    ProgressEvent::PdfPageProgress { task_id, .. } => {
        if task_id != &filter_track_id {
            continue; // Skip, not for this upload
        }
    }
    // Other PDF-related events can be added here
    _ => {
        continue; // Skip non-PDF events on this filtered endpoint
    }
}
```

## Route Design

| Endpoint | Purpose |
|----------|---------|
| `GET /ws/pipeline/progress` | All events (admin/monitoring) |
| `GET /ws/progress/{track_id}` | Filtered PDF progress for specific upload |
