# Database Migration Fix - Critical Security Update

**Date:** 2026-01-28  
**Issue:** Duplicate migration files and missing tenant isolation  
**Severity:** CRITICAL - Data loss risk + Security vulnerability

## Problem Summary

The EdgeQuake database migrations had **THREE critical issues**:

1. **Duplicate Migration Files** with conflicting numbers:
   - `014_add_tenant_workspace_to_tasks.sql` (WRONG - conflicts with 014_add_graph_indexes.sql)
   - `015_add_tenant_workspace_to_tasks.sql` (WRONG - conflicts with 015_add_fulltext_search.sql)
   - `018_add_tenant_workspace_to_tasks.sql` (OLD VERSION - referenced wrong column)

2. **Wrong Column Reference**: Migrations tried to extract tenant_id from `payload` column, but the tasks table uses `task_data` column (verified in `002_add_tasks_table.sql` line 145).

3. **Non-Idempotent Migrations**: Running migrations twice would fail, breaking existing deployments.

## Solution Implemented

### 1. Cleaned Up Duplicate Files

- **REMOVED:** `014_add_tenant_workspace_to_tasks.sql`
- **REMOVED:** `015_add_tenant_workspace_to_tasks.sql`
- **CREATED:** `019_add_tenant_workspace_to_tasks.sql` (bulletproof version)

### 2. Fixed Column Reference

Changed from:

```sql
(payload->>'tenant_id')::UUID  -- WRONG - column doesn't exist
```

To:

```sql
(task_data->>'tenant_id')::UUID  -- CORRECT - matches 002 migration
```

### 3. Made Migration Bulletproof

The new `019_add_tenant_workspace_to_tasks.sql` includes:

#### Safety Features:

- ✅ **Idempotent**: Can run multiple times safely (uses `IF NOT EXISTS`)
- ✅ **Non-Destructive**: Adds columns with NULL first, migrates data, then adds constraints
- ✅ **Validated**: Checks for NULL values before adding NOT NULL constraints
- ✅ **Defensive**: Handles missing task_data gracefully with COALESCE
- ✅ **Fallback**: Uses default tenant (00000000-0000-0000-0000-000000000001) for orphaned tasks
- ✅ **Comprehensive Logging**: RAISE NOTICE for every step
- ✅ **Error Handling**: Exception blocks for optional policies

#### Migration Steps:

1. **Add Columns** - tenant_id and workspace_id (NULL initially)
2. **Migrate Data** - Extract from task_data JSON or use default tenant
3. **Validate** - Check all rows have tenant/workspace before continuing
4. **Add Constraints** - Set NOT NULL after validation passes
5. **Create Indexes** - Performance indexes for tenant/workspace queries
6. **Enable RLS** - Row Level Security policies for defense-in-depth
7. **Final Validation** - Count tasks and tenants, print summary

## Testing Requirements

### Before Deploying to Production:

1. **Test on Fresh Database:**

   ```bash
   docker rm -f edgequake-postgres && docker volume rm docker_postgres-data
   make backend-bg
   # Check logs: tail -f /tmp/edgequake-backend.log
   ```

2. **Test on Database with Existing Tasks:**

   ```sql
   -- Create test tasks WITHOUT tenant_id/workspace_id
   INSERT INTO tasks (track_id, task_type, status, task_data, metadata)
   VALUES ('test-1', 'upload', 'pending', '{}', '{}');

   -- Run migration
   \i 019_add_tenant_workspace_to_tasks.sql

   -- Verify default tenant was assigned
   SELECT track_id, tenant_id, workspace_id FROM tasks WHERE track_id = 'test-1';
   ```

3. **Test Idempotency:**

   ```sql
   -- Run migration again - should not fail
   \i 019_add_tenant_workspace_to_tasks.sql
   ```

4. **Test RLS Policies:**

   ```sql
   -- Set tenant context
   SET app.current_tenant_id = '00000000-0000-0000-0000-000000000002';
   SET app.current_workspace_id = 'b6ef94c5-c621-4dff-b0fc-7562dfc9cabb';

   -- Should only see tasks from this tenant/workspace
   SELECT * FROM tasks;
   ```

## Deployment Checklist

- [ ] **BACKUP DATABASE** before running migration
- [ ] Test migration on staging environment first
- [ ] Verify `task_data` column exists in tasks table
- [ ] Check for existing tasks without tenant_id/workspace_id
- [ ] Plan default tenant strategy for orphaned tasks
- [ ] Schedule maintenance window (migration adds indexes)
- [ ] Monitor migration logs for errors
- [ ] Validate tenant isolation after migration
- [ ] Test application functionality post-migration
- [ ] Document default tenant ID for support team

## Rollback Plan

If migration fails:

```sql
-- Remove constraints (if added)
ALTER TABLE tasks ALTER COLUMN tenant_id DROP NOT NULL;
ALTER TABLE tasks ALTER COLUMN workspace_id DROP NOT NULL;

-- Drop indexes
DROP INDEX IF EXISTS idx_tasks_tenant_workspace;
DROP INDEX IF EXISTS idx_tasks_tenant_id;
DROP INDEX IF EXISTS idx_tasks_workspace_id;
DROP INDEX IF EXISTS idx_tasks_tenant_workspace_status;

-- Drop RLS policies
DROP POLICY IF EXISTS tasks_tenant_isolation ON tasks;
DROP POLICY IF EXISTS tasks_service_role_all ON tasks;
ALTER TABLE tasks DISABLE ROW LEVEL SECURITY;

-- Remove columns (CAREFUL - this loses data!)
-- ALTER TABLE tasks DROP COLUMN tenant_id;
-- ALTER TABLE tasks DROP COLUMN workspace_id;

-- Restore from backup instead
```

## Security Impact

**Before Fix:**

- Tasks visible across all tenants (CRITICAL vulnerability)
- Statistics showed counts from all tenants
- No database-level tenant isolation

**After Fix:**

- ✅ Tasks isolated by tenant_id AND workspace_id
- ✅ Statistics filtered per tenant/workspace
- ✅ Row Level Security enforces isolation at database level
- ✅ Service role bypass for maintenance operations

## Performance Impact

**New Indexes Added:**

- `idx_tasks_tenant_workspace` - Composite (tenant_id, workspace_id, created_at DESC)
- `idx_tasks_tenant_id` - Single column (tenant_id)
- `idx_tasks_workspace_id` - Single column (workspace_id)
- `idx_tasks_tenant_workspace_status` - Composite (tenant_id, workspace_id, status)

**Expected Impact:**

- Index creation time: ~1-5 seconds per 10,000 tasks
- Query performance: **IMPROVED** (proper filtering now possible)
- Storage overhead: ~1-2% additional disk space

## Related Changes

This migration works together with the code fix:

- **Backend:** Updated `get_statistics()` to accept and use TaskFilter
- **Storage:** PostgreSQL and Memory implementations filter by tenant/workspace
- **API Handlers:** Pass tenant context to statistics queries

See commit: `Fix tenant isolation in get_statistics`

## References

- Original tasks table schema: `002_add_tasks_table.sql`
- RLS policies: `009_add_rls_policies.sql`
- Multi-tenancy setup: `008_add_multi_tenancy_tables.sql`
- Code fix: [edgequake-tasks](../crates/edgequake-tasks/src/)

---

**CRITICAL:** This migration must be tested on a database copy before production deployment. Data loss is not acceptable.
