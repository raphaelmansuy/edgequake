-- PDF lists order by created_at DESC, then pdf_id. A deep page uses OFFSET,
-- so idx_pdf_documents_workspace_created reads every skipped row. On 15,000
-- PDFs in one workspace, OFFSET 14950 LIMIT 50 reads all 15,000 (4.036 ms,
-- 825 buffers). A keyset on the global created_at index stops when nothing
-- older belongs to another workspace (0.086 ms, 1 row filtered). With 20,000
-- foreign rows just older than the cursor, that same generic plan removes
-- 19,999 rows (6.655 ms, 40,247 buffers).
--
-- `created_at <= cursor AND (created_at, pdf_id) < (cursor, id)` is an index
-- condition on these indexes (0.029 ms, 7 buffers on that foreign set; status
-- page 0.047 ms). The id column keeps a tie from walking the whole group.
-- Expand only. The older indexes stay.

CREATE INDEX IF NOT EXISTS idx_pdf_documents_workspace_created_id
    ON public.pdf_documents (workspace_id, created_at DESC, pdf_id DESC);

CREATE INDEX IF NOT EXISTS idx_pdf_documents_workspace_status_created_id
    ON public.pdf_documents (workspace_id, processing_status, created_at DESC, pdf_id DESC);

CREATE INDEX IF NOT EXISTS idx_pdf_documents_created_id
    ON public.pdf_documents (created_at DESC, pdf_id DESC);

CREATE INDEX IF NOT EXISTS idx_pdf_documents_status_created_id
    ON public.pdf_documents (processing_status, created_at DESC, pdf_id DESC);
