-- A document list filtered by status used to read the newest rows and
-- drop the ones that do not match. On 80,000 indexed documents, the 50
-- oldest `failed` rows are the answer. `idx_documents_created_at` with
-- `status = $2` removes those 79,950 newer rows (8.523 ms warm, 2,558
-- buffers; the first run was 35.847 ms). The list then kept none of them
-- once it had only looked at the newest 2,001.
--
-- `idx_documents_workspace_status_created` is
-- `(workspace_id, status, created_at DESC, id DESC)`. A generic plan
-- index-only scans it and stops at the page (0.021 ms, 14 buffers, 50
-- rows). The tenant arm uses
-- `idx_documents_workspace_tenant_status_created`. Rows with no tenant
-- use `idx_documents_workspace_null_tenant_status_created`.

CREATE INDEX IF NOT EXISTS idx_documents_workspace_status_created
    ON public.documents (workspace_id, status, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_documents_workspace_tenant_status_created
    ON public.documents (workspace_id, tenant_id, status, created_at DESC, id DESC)
    WHERE tenant_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_workspace_null_tenant_status_created
    ON public.documents (workspace_id, status, created_at DESC, id DESC)
    WHERE tenant_id IS NULL;
