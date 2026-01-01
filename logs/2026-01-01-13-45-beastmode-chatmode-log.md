# Task Log - 2026-01-01

## Actions
- Fixed `BoundingBox` construction in `lattice.rs` (switched to `from_xywh`).
- Fixed `sort_by` panic in `lattice.rs` by implementing stable, transitive comparison using discrete row bucketing.
- Verified PDF extraction with `convert_one_tool` example (panic resolved, tables detected).
- Implemented Phase 3 (Stream Engine) backend:
    - Modified `PipelineState` in `edgequake-tasks` to support event broadcasting.
    - Added `PipelineEvent` enum for structured updates.
    - Added SSE endpoint `stream_pipeline_status` in `edgequake-api`.
    - Registered `/api/v1/pipeline/stream` route.
    - Enabled `sync` feature for `tokio-stream`.

## Decisions
- Used `tokio::sync::broadcast` for distributing pipeline events to multiple SSE clients.
- Defined a `PipelineEvent` enum to encapsulate different types of updates (Log, Progress, StateChange).
- Used discrete bucketing (`(y / 5.0).round() as i32`) for sorting text elements to ensure stability and prevent panics.

## Next Steps
- Implement the frontend consumer for the SSE stream in `edgequake_webui`.
- Add unit tests for the SSE handler.
- Verify end-to-end with a real job execution.

## Lessons
- `slice::sort_by` in Rust requires strict total ordering; floating point comparisons must be handled carefully to avoid panics.
- `tokio-stream` requires the `sync` feature for `BroadcastStream`.
