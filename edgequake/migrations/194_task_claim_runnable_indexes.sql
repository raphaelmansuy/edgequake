-- SPEC-164: a fair claim must not walk older rows a hold or a live lease
-- has already ruled out.
--
-- `fairness_hold_until IS NULL OR fairness_hold_until <= now()` and
-- `lease_expires_at IS NULL OR lease_expires_at < now()` hide each predicate
-- from a generic plan, so the sample walks the backlog in created_at order.
-- The claim splits those predicates. These partial indexes start at the
-- runnable rows and stop at the page. A due hold still uses
-- idx_tasks_fairness_hold_until. Expand only. Does not raise the serving
-- floor. Apache AGE stays dropped at migration 171.

CREATE INDEX IF NOT EXISTS idx_tasks_pending_unheld_created
    ON public.tasks (created_at, track_id)
    WHERE status = 'pending'
      AND fairness_parked_at IS NULL
      AND fairness_hold_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_pending_unheld_workspace
    ON public.tasks (workspace_id, created_at, track_id)
    WHERE status = 'pending'
      AND fairness_parked_at IS NULL
      AND fairness_hold_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_processing_unleased_created
    ON public.tasks (created_at, track_id)
    WHERE status = 'processing'
      AND lease_expires_at IS NULL
      AND fairness_hold_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_processing_unleased_workspace
    ON public.tasks (workspace_id, created_at, track_id)
    WHERE status = 'processing'
      AND lease_expires_at IS NULL
      AND fairness_hold_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_processing_expired_unheld
    ON public.tasks (lease_expires_at, created_at, track_id)
    WHERE status = 'processing'
      AND lease_expires_at IS NOT NULL
      AND fairness_hold_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_processing_expired_unheld_workspace
    ON public.tasks (workspace_id, lease_expires_at, created_at, track_id)
    WHERE status = 'processing'
      AND lease_expires_at IS NOT NULL
      AND fairness_hold_until IS NULL;
