# OODA Iteration 17 - Decide Phase

## Decision: Fix Task Status Database Constraint

### Selected Fix: Option A - Update Database Constraint

**Rationale:**

1. Rust enum values ('processing', 'indexed') are more semantic
2. API already uses these values - no breaking changes
3. Database is internal storage - should conform to application code

### Implementation Plan

#### Step 1: Create Migration File

Create `edgequake/migrations/024_fix_task_status_constraint.sql`:

```sql
-- OODA-17: Fix task status constraint to match Rust enum values
--
-- Problem: Rust TaskStatus enum uses 'processing' and 'indexed' but
-- database constraint expected 'running' and 'completed'.
--
-- Impact: ALL task status updates were failing silently.

-- Update any existing data using old values
UPDATE tasks SET status = 'processing' WHERE status = 'running';
UPDATE tasks SET status = 'indexed' WHERE status = 'completed';

-- Drop old constraint and add new one
ALTER TABLE tasks
DROP CONSTRAINT IF EXISTS tasks_valid_status;

ALTER TABLE tasks
ADD CONSTRAINT tasks_valid_status
  CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled'));
```

#### Step 2: Apply Migration

```bash
psql -U edgequake -d edgequake < edgequake/migrations/024_fix_task_status_constraint.sql
```

#### Step 3: Verify Fix

1. Restart backend
2. Upload a test document
3. Verify no constraint errors in logs
4. Verify task status updates properly

### Expected Outcome

| Check              | Before | After   |
| ------------------ | ------ | ------- |
| Task status update | ERROR  | SUCCESS |
| Processing status  | Fails  | Works   |
| Indexed status     | Fails  | Works   |

### Rollback Plan

If issues occur:

```sql
-- Rollback to old constraint
UPDATE tasks SET status = 'running' WHERE status = 'processing';
UPDATE tasks SET status = 'completed' WHERE status = 'indexed';

ALTER TABLE tasks
DROP CONSTRAINT IF EXISTS tasks_valid_status;

ALTER TABLE tasks
ADD CONSTRAINT tasks_valid_status
  CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'));
```

### Acceptance Criteria

- [ ] Migration file created
- [ ] Migration applied successfully
- [ ] No constraint errors in logs
- [ ] Task status transitions work: pending → processing → indexed
- [ ] Tests pass
