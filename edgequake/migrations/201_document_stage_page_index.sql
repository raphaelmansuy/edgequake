-- A document list for the pending or processing chip also matches
-- `metadata->>'current_stage'`. On 80,000 indexed documents, the 50 oldest
-- rows with `current_stage = 'queued'` are the answer. The OR
-- `status IN ('pending','queued') OR current_stage = 'queued'` uses
-- `idx_documents_created_at` and removes those 79,950 newer rows
-- (12.595 ms warm, 2,558 buffers). The union of one equality per value
-- stops at the page (0.055 ms warm, 17 buffers, 50 rows).
--
-- `current_stage = ANY($stages)` is not an index condition on a generic
-- plan (10.739 ms, the same 79,950 rows removed). One literal equality per
-- stage is. `idx_documents_workspace_stage_created` is
-- `(workspace_id, (metadata->>'current_stage'), created_at DESC, id DESC)`
-- where the stage is present. That arm stops at the page.
-- Status arms reuse migration 200. The tenant arm uses
-- `idx_documents_workspace_tenant_stage_created`. Rows with no tenant use
-- `idx_documents_workspace_null_tenant_stage_created`.

CREATE INDEX IF NOT EXISTS idx_documents_workspace_stage_created
    ON public.documents (workspace_id, (metadata->>'current_stage'), created_at DESC, id DESC)
    WHERE (metadata->>'current_stage') IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_workspace_tenant_stage_created
    ON public.documents (workspace_id, tenant_id, (metadata->>'current_stage'), created_at DESC, id DESC)
    WHERE tenant_id IS NOT NULL
      AND (metadata->>'current_stage') IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_workspace_null_tenant_stage_created
    ON public.documents (workspace_id, (metadata->>'current_stage'), created_at DESC, id DESC)
    WHERE tenant_id IS NULL
      AND (metadata->>'current_stage') IS NOT NULL;
