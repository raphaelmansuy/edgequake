-- Entity search with a modality filter was
-- `$6 IS NULL OR metadata->>'modality' = ANY($6)`.
-- A generic plan seq-scanned 40000 entities and removed 39980
-- (8.383 ms warm, 2503 buffers; the first cold run was 13.999 ms).
-- This index starts at the matching modality (0.634 ms, 83 buffers,
-- 20 chart rows). The statement requires the predicate only when
-- the caller passes modalities. An unfiltered search keeps the
-- open statement. Workspace and tenant stay optional.

CREATE INDEX IF NOT EXISTS idx_entities_modality
    ON public.entities ((metadata->>'modality'))
    WHERE (metadata->>'modality') IS NOT NULL;
