-- Task lists are `WHERE tenant_id = $1 AND workspace_id = $2
-- ORDER BY created_at DESC, track_id DESC`.
-- `idx_tasks_created` is global, so a newer task in another workspace is
-- walked and discarded. The keyset page uses the same order with a row
-- comparison, which this index can stop at.

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_created_track
    ON public.tasks (tenant_id, workspace_id, created_at DESC, track_id DESC);
