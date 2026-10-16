-- Entity ANN limited to one document is
-- `e.source_ids && $4 ORDER BY embedding <=> probe LIMIT k`.
-- The previous statement was `$4 IS NULL OR e.source_ids && $4`, so a
-- generic plan seq-scanned every entity and then kept the overlapping
-- rows. On 40,000 entities and 20 that carry the document, that removed
-- 39,980 rows (7.363 ms, 1,721 buffers). Requiring the predicate without
-- this index still seq-scans (6.850 ms). This index starts the page at
-- the overlapping entities (0.279 ms, 83 buffers, 20 rows).
-- The statement requires the overlap only when the caller passes
-- documents. An unfiltered search keeps the previous statement.
-- Expand only. This does not raise the serving floor.
-- Apache AGE stays dropped at migration 171.

CREATE INDEX IF NOT EXISTS idx_entities_source_ids
    ON public.entities USING gin (source_ids);
