-- Chart chunk search is
-- `content_tsv @@ websearch_to_tsquery(...) AND metadata->>'modality' = ANY($6)`.
-- The previous statement was `$6 IS NULL OR metadata->>'modality' = ANY($6)`,
-- so a generic plan scored every chunk that matched the words and then
-- threw away the other modalities. On 2,000 chunks that share the term
-- and 20 chart rows, that removed 1,980 rows (2.908 ms, 621 buffers).
-- This index starts the page at the chart rows (0.225 ms, 87 buffers).
-- The statement requires the modality predicate only when the caller
-- passes modalities. An unfiltered search keeps the previous statement.

CREATE INDEX IF NOT EXISTS idx_chunks_modality
    ON public.chunks ((metadata->>'modality'))
    WHERE (metadata->>'modality') IS NOT NULL;
