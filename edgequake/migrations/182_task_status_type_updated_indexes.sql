-- Status and task-type pages sorted by update time are
-- `WHERE tenant_id = $1 AND workspace_id = $2 AND status = $3 ORDER BY updated_at DESC`
-- (or `task_type = $3`). Migration 177 serves that filter with `created_at`.
-- Migration 178 is `(tenant_id, workspace_id, updated_at DESC, track_id DESC)`,
-- so a page of older failed rows walks newer rows and throws them away. On
-- 80,000 pending tasks plus 40,000 older failed tasks, that filter removed
-- 80,000 rows (11.424 ms, 17,340 buffers). These indexes stop at LIMIT
-- (0.054 ms and 0.049 ms, 11 buffers). The updated_at sort has no track_id
-- tie-break, so these indexes do not add one.

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_status_updated
    ON public.tasks (tenant_id, workspace_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_type_updated
    ON public.tasks (tenant_id, workspace_id, task_type, updated_at DESC);
