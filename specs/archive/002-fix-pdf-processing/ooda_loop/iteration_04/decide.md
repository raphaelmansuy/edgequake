# OODA-04 Decide Phase

## Date: 2026-01-31
## Decisions Made

## Decision 1: Apply safe_truncate() to All Affected Files

**Choice**: Add local `safe_truncate()` helper to each file with unsafe slicing

**Rationale**:
- Consistent with OODA-03 fix pattern
- Each file is self-contained (no cross-module dependencies)
- Easy to audit and maintain
- Zero performance overhead

**Alternatives Rejected**:
- Shared utility crate → adds complexity for simple helper
- Macro-based solution → harder to debug
- Remove debug logging → loses valuable diagnostics

## Decision 2: Capture Tokio Handle at Construction

**Choice**: Store `tokio::runtime::Handle` in struct, capture in `new()`

**Rationale**:
- `Handle::current()` is cheap and infallible when in runtime context
- The callback is always constructed from Axum handler (Tokio context)
- `self.runtime_handle.spawn()` works from any thread
- Standard Tokio pattern for sync-async bridging

**Alternatives Rejected**:
- `block_on()` → blocks Rayon threads, poor performance
- `tokio::task::spawn_local()` → requires LocalSet, more complex
- Remove async operations → breaks WebSocket updates

## Decision 3: Fix All Four spawn() Locations

**Choice**: Replace all `tokio::spawn` calls in pipeline_progress_callback.rs

**Locations**:
1. Line 165: `on_extraction_start()`
2. Line 250: `on_extraction_progress()`  
3. Line 297: `on_extraction_complete()`
4. Line 352: `on_extraction_error()`

**Rationale**:
- All four callbacks run from Rayon context
- Consistent fix prevents future panics
- Easy code review (same pattern everywhere)

## Decision 4: Keep WHY Comments

**Choice**: Add WHY comments explaining the pattern

**Example**:
```rust
// WHY: Store handle to spawn async tasks from sync Rayon context
runtime_handle: Handle,
```

**Rationale**:
- Future maintainers understand the constraint
- Prevents accidental removal
- Documents non-obvious technical requirement

## Implementation Checklist

- [x] Add safe_truncate() to block_builder.rs
- [x] Add safe_truncate() to layout_processing.rs  
- [x] Add safe_truncate() to text_cleanup.rs
- [x] Add Handle import to pipeline_progress_callback.rs
- [x] Add runtime_handle field with WHY comment
- [x] Capture handle in new()
- [x] Replace 4x tokio::spawn with self.runtime_handle.spawn
- [x] Rebuild and verify 0 panics
