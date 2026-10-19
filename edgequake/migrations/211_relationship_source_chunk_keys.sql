-- Relationship document lineage never reached `relationships`.
--
-- Document-scoped relationship search required
-- `relationships.source_chunk_ids && (chunk uuids of the documents)`, but that
-- uuid[] column is NULL on every row: the merger writes relationships in the
-- merge phase, before the persist phase creates the chunk rows, so no chunk
-- uuid exists yet when the row is written. A document-scoped search therefore
-- matched nothing. (Entities were never affected by that ordering: they carry
-- the legacy chunk KEYS in `entities.source_chunk_ids`.)
--
-- A chunk key is known at write time. This column holds it, exactly like the
-- entity column, and is mapped to documents through
-- `chunks.legacy_chunk_key` at search time. Additive and nullable: no table
-- rewrite. Migration 212 backfills existing rows from edge lineage.
-- `source_chunk_ids` (uuid[]) stays for compatibility and is no longer read.

ALTER TABLE public.relationships
    ADD COLUMN IF NOT EXISTS source_chunk_keys text[];

COMMENT ON COLUMN public.relationships.source_chunk_keys IS
    'Legacy chunk keys ("{doc_id}-chunk-{n}") that produced this relationship; same shape as entities.source_chunk_ids. Maps to documents via chunks.legacy_chunk_key.';

CREATE INDEX IF NOT EXISTS idx_relationships_source_chunk_keys
    ON public.relationships USING gin (source_chunk_keys);
