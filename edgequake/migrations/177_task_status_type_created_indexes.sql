-- Status and task-type pages are
-- `WHERE tenant_id = $1 AND workspace_id = $2 AND status = $3`
-- (or `task_type = $3`) `ORDER BY created_at DESC, track_id DESC`.
-- The time index from 175 walks newer rows of other statuses or types and
-- discards them. On a workspace of 80k pending plus 40k older failed tasks,
-- the failed page removed 80000 rows (16.198 ms). These indexes stop at LIMIT.
-- The keyset comparison on (created_at, track_id) uses the same column order.

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_status_created_track
    ON public.tasks (tenant_id, workspace_id, status, created_at DESC, track_id DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_type_created_track
    ON public.tasks (tenant_id, workspace_id, task_type, created_at DESC, track_id DESC);
