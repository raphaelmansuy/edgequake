-- PDF lists are `WHERE workspace_id = $1 ORDER BY created_at DESC`.
-- `idx_pdf_documents_created` is global, so a newer foreign workspace is
-- walked and discarded. `idx_pdf_documents_workspace_status` cannot serve
-- that order: `processing_status` sits between `workspace_id` and `created_at`.

CREATE INDEX IF NOT EXISTS idx_pdf_documents_workspace_created
    ON public.pdf_documents (workspace_id, created_at DESC);
