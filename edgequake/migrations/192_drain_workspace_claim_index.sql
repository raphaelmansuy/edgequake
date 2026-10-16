-- A workspace-scoped drain claims the next 50 due rows for that workspace.
-- The generic plan of `$2 IS NULL OR workspace_id = $2` cannot start there.
--
-- Outbox: on 79,550 unprocessed events, 79,500 of them older and owned by
-- another workspace, that plan walked `idx_outbox_events_unprocessed` and
-- removed 79,500 rows (10.054 ms, 1,369 buffers). Requiring
-- `workspace_id = $2` uses this index and stops at 50 (0.042 ms, 4 buffers).
-- A drain with no workspace keeps `idx_outbox_events_unprocessed`.
--
-- Quarantine: the same optional predicate seq-scanned 79,550 due rows
-- (8.153 ms, 1,346 buffers) and still did when the workspace predicate was
-- removed, because `(status, next_attempt_at)` cannot order both pending and
-- failed. `(next_attempt_at)` under the same status predicate stops at 50
-- (0.086 ms). The workspace index serves a scoped claim (0.037 ms, 4 buffers).

CREATE INDEX IF NOT EXISTS idx_outbox_events_workspace_unprocessed
    ON public.outbox_events (workspace_id, available_at, created_at)
    WHERE processed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_compensation_quarantine_due
    ON public.compensation_quarantine (next_attempt_at)
    WHERE status IN ('pending', 'failed');

CREATE INDEX IF NOT EXISTS idx_compensation_quarantine_workspace_next
    ON public.compensation_quarantine (workspace_id, next_attempt_at)
    WHERE status IN ('pending', 'failed');
