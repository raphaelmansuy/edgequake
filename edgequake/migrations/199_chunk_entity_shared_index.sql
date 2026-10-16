-- Document lineage asks which of a document's entities also appear on
-- another document:
-- `workspace_id = $1 AND entity_name = ANY($2)` then
-- `COUNT(DISTINCT split_part(chunk_id, '-chunk-', 1))`.
-- `idx_cel_entity_workspace` is (entity_name, workspace_id), so the
-- count fetches every matching heap row. On 80000 links, two names
-- (1600 rows) took 1.859 ms and 817 buffers. This covering index
-- answers the same count with an index-only scan (0.763 ms, 26
-- buffers, 0 heap fetches).

CREATE INDEX IF NOT EXISTS idx_cel_entity_workspace_chunk
    ON public.chunk_entity_links (entity_name, workspace_id, chunk_id);
