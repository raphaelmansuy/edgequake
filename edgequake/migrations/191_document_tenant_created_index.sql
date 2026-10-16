-- The documents page is
-- `workspace_id = $1 AND tenant_id = $2 ORDER BY created_at DESC LIMIT 50`,
-- plus rows whose tenant is unset.
-- A generic plan cannot use a composite index for
-- `$2 IS NULL OR tenant_id IS NULL OR tenant_id = $2`, so it walks
-- `idx_documents_created_at` and drops every newer row from other tenants.
-- On 80,000 documents, 79,500 of them newer and owned by someone else, that
-- removed 79,500 rows (15.106 ms, 2,449 buffers).
-- The equality arm stops on this index (0.041 ms, 7 buffers, 50 rows).
-- `id DESC` matches the page's tie-break. Without it the planner still
-- prefers the global `created_at` index.
-- The partial index serves the unset-tenant arm in the same order, so a
-- page stops after LIMIT even when most rows have no tenant.

CREATE INDEX IF NOT EXISTS idx_documents_workspace_tenant_created
    ON public.documents (workspace_id, tenant_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_documents_workspace_null_tenant_created
    ON public.documents (workspace_id, created_at DESC, id DESC)
    WHERE tenant_id IS NULL;
