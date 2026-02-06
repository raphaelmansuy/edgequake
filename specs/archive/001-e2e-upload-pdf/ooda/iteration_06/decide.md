# OODA Iteration 06 - Decide

**Date**: 2026-02-06
**Focus**: Implementation Plan for PostgreSQL Task Storage

## Decision Summary

Replace `MemoryTaskStorage` with `PostgresTaskStorage` in `AppState::new_postgres()`.

## Changes Required

### Change 1: state.rs Line 793

**File**: `edgequake/crates/edgequake-api/src/state.rs`
**Location**: Inside `new_postgres()` function, around line 793

**Before**:

```rust
// Create task infrastructure
let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
```

**After**:

```rust
// Create task infrastructure (OODA-06: Use PostgreSQL for task persistence)
// WHY: Tasks must persist across backend restarts so cancel/retry work correctly
let task_storage: SharedTaskStorage = Arc::new(
    edgequake_tasks::postgres::PostgresTaskStorage::new(pool.clone())
);
```

### Change 2: Add Log Statement

Add logging to confirm PostgreSQL task storage is used:

```rust
tracing::info!("✓ Task storage: PostgreSQL (persistent across restarts)");
```

## Non-Changes

- **MemoryTaskStorage stays for test/memory mode**: `new_memory()` and `test_state()` should continue using memory storage for simplicity and speed.
- **No migration changes**: The `tasks` table already exists from migration 002.
- **No API changes**: TaskStorage trait is already abstracted.

## Implementation Order

1. Read current state.rs to verify exact line numbers
2. Make the replacement
3. Build to verify compilation
4. Test backend starts correctly
5. Verify tasks are written to PostgreSQL

## Acceptance Criteria

- [ ] Backend starts without errors
- [ ] Health endpoint returns healthy
- [ ] Uploading document creates task in PostgreSQL `tasks` table
- [ ] Restarting backend preserves tasks
- [ ] Cancel works on persisted tasks
