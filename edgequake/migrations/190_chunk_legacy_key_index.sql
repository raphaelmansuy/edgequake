-- Chunk search filtered by id is
-- `content_tsv @@ websearch_to_tsquery(...) AND (id = ANY OR legacy key = ANY)`.
-- The previous statement was `$7 IS NULL OR ...`, so a generic plan scored
-- every chunk that matched the words and then kept the requested id.
-- On 2,000 chunks that share the term and one legacy key, that removed
-- 1,999 rows (2.287 ms, 392 buffers). The search now looks each id up
-- first. This index serves the legacy-key arm (0.112 ms, 7 buffers).
-- A UUID uses the primary key (0.067 ms, 9 buffers).

CREATE INDEX IF NOT EXISTS idx_chunks_legacy_chunk_key
    ON public.chunks ((metadata->>'legacy_chunk_key'))
    WHERE (metadata->>'legacy_chunk_key') IS NOT NULL;
