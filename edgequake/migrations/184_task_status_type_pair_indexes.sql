-- A task page can filter status and task_type together:
-- `WHERE tenant_id = $1 AND workspace_id = $2 AND status = $3 AND task_type = $4
--  ORDER BY created_at DESC, track_id DESC` (or `ORDER BY updated_at DESC`).
-- Migration 177 serves status or type alone, ordered by created_at.
-- Migration 182 does the same for updated_at. With both filters set, the
-- planner uses one of those indexes and throws away rows that fail the other
-- predicate. On 80,000 pending tasks plus 40,000 newer failed inserts and
-- 40,000 newer pending scans, a page of older failed scans removed 40,000
-- rows (5.624 ms, 956 buffers created_at; 5.402 ms, 867 buffers updated_at).
-- These indexes stop at LIMIT (0.068 ms, 5 buffers and 0.050 ms, 6 buffers).
-- A generic plan of `status = $3 AND task_type = $4` uses them. A page that
-- filters only status still uses the migration 177 index. The list SQL is
-- unchanged. The updated_at sort has no track_id tie-break, so that index
-- does not add one.

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_status_type_created_track
    ON public.tasks (tenant_id, workspace_id, status, task_type, created_at DESC, track_id DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_status_type_updated
    ON public.tasks (tenant_id, workspace_id, status, task_type, updated_at DESC);
