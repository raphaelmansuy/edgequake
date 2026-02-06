# OODA Iteration 06 - Orient

**Date**: 2026-02-06
**Focus**: Root Cause Analysis and Solution Design

## First Principles Analysis

### Why Tasks Must Persist

1. **State Consistency**: Document metadata references track_id. If task is lost, the reference is broken.
2. **Cancel/Retry**: Users expect to cancel or retry processing. Without persisted tasks, these operations fail.
3. **Monitoring**: Operators need task history for debugging and capacity planning.
4. **Idempotency**: Reprocessing detection requires knowing what tasks have run.

### Why Memory Storage Was Used

Likely historical reasons:

- Initial development used memory for simplicity
- PostgresTaskStorage was added later but state.rs wasn't updated
- The discrepancy wasn't caught in testing because tasks worked during a single session

## Solution Options

### Option A: Direct Replacement (Recommended)

**Effort**: Low (5 lines changed)
**Risk**: Low (PostgresTaskStorage already tested)
**Impact**: High (fixes root cause)

Replace `MemoryTaskStorage::new()` with `PostgresTaskStorage::new(pool.clone())` in `new_postgres()`.

### Option B: Factory Pattern

**Effort**: Medium
**Risk**: Medium (new code)
**Impact**: High

Create `TaskStorageFactory` that auto-selects based on config.

```rust
pub fn create_task_storage(config: &StorageConfig) -> SharedTaskStorage {
    match config.mode {
        StorageMode::Memory => Arc::new(MemoryTaskStorage::new()),
        StorageMode::PostgreSQL(pool) => Arc::new(PostgresTaskStorage::new(pool)),
    }
}
```

Rejected: Over-engineering for single use case.

### Option C: Environment Variable Switch

**Effort**: Low
**Risk**: Low
**Impact**: Medium (adds flexibility but also confusion)

```rust
let task_storage = if env::var("USE_POSTGRES_TASKS").is_ok() {
    Arc::new(PostgresTaskStorage::new(pool.clone()))
} else {
    Arc::new(MemoryTaskStorage::new())
};
```

Rejected: No reason to keep memory option in postgres mode.

## Decision: Option A

Direct replacement is the correct approach:

- The entire `new_postgres()` function is for PostgreSQL mode
- All other storage backends already use PostgreSQL
- Task storage should be consistent with other storage

## Risk Assessment

| Risk                     | Likelihood | Impact | Mitigation                               |
| ------------------------ | ---------- | ------ | ---------------------------------------- |
| PostgresTaskStorage bugs | Low        | High   | Already tested, has unit tests           |
| Migration issues         | Low        | Medium | Migration 002 already applied            |
| Performance regression   | Low        | Low    | PG is designed for this; tasks are small |

## Verification Plan

1. Build and start backend
2. Check logs for task storage initialization
3. Upload a test document
4. Verify task in PostgreSQL: `SELECT * FROM tasks LIMIT 1;`
5. Restart backend
6. Verify task still exists
7. Cancel a processing document
8. Verify both task and document updated
