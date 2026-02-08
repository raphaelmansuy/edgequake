# OODA Iteration 17 - Orient Phase

## Analysis: Task Status Constraint Mismatch

### Root Cause Identified

**The Rust `TaskStatus` enum doesn't match the database constraint.**

| Rust Enum Value | Serialized String | DB Constraint Expects |
| --------------- | ----------------- | --------------------- |
| `Pending`       | "pending"         | 'pending' ✅          |
| `Processing`    | "processing"      | 'running' ✗           |
| `Indexed`       | "indexed"         | 'completed' ✗         |
| `Failed`        | "failed"          | 'failed' ✅           |
| `Cancelled`     | "cancelled"       | 'cancelled' ✅        |

### Code Location

**Rust Enum** (`types.rs:11-21`):

```rust
pub enum TaskStatus {
    Pending,
    Processing,   // Serializes to "processing"
    Indexed,      // Serializes to "indexed"
    Failed,
    Cancelled,
}
```

**Database Constraint** (`001_init_database.sql:299`):

```sql
CONSTRAINT tasks_valid_status CHECK (
  status IN ('pending', 'running', 'completed', 'failed', 'cancelled')
)
```

### Impact

Every time a task transitions to:

- `Processing` → UPDATE fails because DB expects 'running'
- `Indexed` (success) → UPDATE fails because DB expects 'completed'

This explains the errors in logs:

```
Failed to update task status: Storage error: new row for relation "tasks"
violates check constraint "tasks_valid_status"
```

### Proposed Fixes

**Option A: Update Database Constraint** (Recommended)

- Add migration to alter the constraint to accept 'processing' and 'indexed'
- Pros: No code changes, backward compatible
- Cons: Requires database migration

**Option B: Update Rust Enum Serialization**

- Use `#[serde(rename = "running")]` on `Processing`
- Use `#[serde(rename = "completed")]` on `Indexed`
- Pros: No database changes
- Cons: May break API consumers expecting 'processing'/'indexed'

**Option C: Use Both Values in Constraint**

- Accept both old and new values in constraint
- Migrate data in place

### Recommendation

**Option A** - Update database constraint to match Rust enum.

This is the cleanest fix because:

1. The Rust enum values are more semantic ('indexed' means document is ready)
2. The existing API responses use 'processing'/'indexed'
3. Database is the internal storage layer, should conform to code

### Migration Script

```sql
-- migrations/024_fix_task_status_constraint.sql
ALTER TABLE tasks
DROP CONSTRAINT tasks_valid_status,
ADD CONSTRAINT tasks_valid_status
  CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled'));
```

## Risk Assessment

| Risk                                     | Severity | Mitigation                              |
| ---------------------------------------- | -------- | --------------------------------------- |
| Existing data with 'running'/'completed' | Low      | Migration handles both                  |
| Rollback complexity                      | Low      | Simple constraint change                |
| Breaking API changes                     | None     | API already uses 'processing'/'indexed' |
