-- Request-path lookups that still scanned the table for one row.
--
-- documents: `track_id = $1 OR metadata->>'track_id' = $1` cannot use
-- idx_documents_track_id. The metadata arm needs its own expression index.
--
-- chunk_*_links: prefix `chunk_id LIKE '{document}-chunk-%'` is the document
-- lineage fallback. Under a UTF-8 collation a default btree will not turn
-- that LIKE into a range. text_pattern_ops will.

CREATE INDEX IF NOT EXISTS idx_documents_metadata_track_id
    ON public.documents ((metadata->>'track_id'));

CREATE INDEX IF NOT EXISTS idx_cel_workspace_chunk_pattern
    ON public.chunk_entity_links (workspace_id, chunk_id text_pattern_ops);

CREATE INDEX IF NOT EXISTS idx_crl_workspace_chunk_pattern
    ON public.chunk_relation_links (workspace_id, chunk_id text_pattern_ops);
