-- Workspace-scoped chunk search is
-- `c.content_tsv @@ websearch_to_tsquery($1) AND d.workspace_id = $2
--  ORDER BY ts_rank_cd(...) LIMIT $8`.
-- The prepared form is `$2 IS NULL OR d.workspace_id = $2`, so a generic
-- plan cannot treat the workspace as an index condition. It bitmaps every
-- chunk that matches the term, then looks up each document and throws the
-- other workspaces away. On 40,000 chunks in the target workspace plus
-- 2,000 foreign chunks that share the term, that was 2.437 ms and 8,250
-- buffers (2,020 document lookups). `idx_chunks_workspace` is
-- `(workspace_id)`. The scoped statement is
-- `c.workspace_id = $2 OR c.workspace_id IS NULL` plus the same document
-- predicate. A generic plan bitmap-ands this index with the content GIN
-- and fetches the 20 hits (1.009 ms, 170 buffers). Chunks with a null
-- workspace_id still use the document predicate.

CREATE INDEX IF NOT EXISTS idx_chunks_workspace
    ON public.chunks (workspace_id);
