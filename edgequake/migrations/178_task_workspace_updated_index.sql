-- Task lists sorted by update time are
-- `WHERE tenant_id = $1 AND workspace_id = $2 ORDER BY updated_at DESC`.
-- `idx_tasks_updated` is global, so newer tasks in another workspace are
-- walked and discarded. On 80k tasks plus 40k newer foreign-workspace rows
-- that filter removed 40000 rows (2.979 ms, 969 buffers). This index stops
-- at LIMIT. `track_id` is the same tie-break the created_at page already uses.

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_updated_track
    ON public.tasks (tenant_id, workspace_id, updated_at DESC, track_id DESC);
