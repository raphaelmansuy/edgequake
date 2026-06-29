-- ============================================================================
-- Migration 066: Chunk lineage tables (SPEC-032 W-07)
-- Version: 1.0.0
-- Date: 2026-06-29
--
-- PURPOSE:
--   Persist the document→chunk→entity/relation lineage chain that was
--   previously only tracked in-memory during ingestion.
--
--   After this migration the system can answer:
--     Q1: "Which PDF page did entity E come from?"
--     Q2: "Which chunks contributed to entity E's description?"
--     Q3: "Delete document X — which entities are now orphaned?"
--     Q4: "Which entities are shared between documents X and Y?"
--
-- TABLES ADDED:
--   1. chunk_entity_links  — M:M between chunks and entity names
--   2. chunk_relation_links — M:M between chunks and source→target relations
--
-- COLUMNS ADDED TO chunks:
--   char_start, char_end, page_start, page_end, embedding_id
--
-- ASCENDING COMPATIBILITY:
--   * All changes are ADDITIVE (new tables / nullable columns).
--   * Existing data is unaffected.
--   * IDEMPOTENT: safe to re-run (all DDL uses IF NOT EXISTS).
--
-- ROLLBACK:
--   DROP TABLE IF EXISTS chunk_relation_links;
--   DROP TABLE IF EXISTS chunk_entity_links;
--   ALTER TABLE chunks DROP COLUMN IF EXISTS char_start;
--   ALTER TABLE chunks DROP COLUMN IF EXISTS char_end;
--   ALTER TABLE chunks DROP COLUMN IF EXISTS page_start;
--   ALTER TABLE chunks DROP COLUMN IF EXISTS page_end;
--   ALTER TABLE chunks DROP COLUMN IF EXISTS embedding_id;
-- ============================================================================

SET search_path = public;

-- ============================================================================
-- STEP 1: Add span columns to chunks table
-- WHY: Enables "which PDF page is this chunk from?" queries (UC-L1).
-- WHY nullable: backfilled lazily on next re-ingestion; old rows stay valid.
-- ============================================================================

ALTER TABLE chunks
    ADD COLUMN IF NOT EXISTS char_start   INT,
    ADD COLUMN IF NOT EXISTS char_end     INT,
    ADD COLUMN IF NOT EXISTS page_start   INT,
    ADD COLUMN IF NOT EXISTS page_end     INT,
    -- embedding_id links the chunk to its vector in eq_*_vectors
    ADD COLUMN IF NOT EXISTS embedding_id TEXT;

-- Index for page-range lookups (common in citation queries)
CREATE INDEX IF NOT EXISTS idx_chunks_page_span
    ON chunks (document_id, page_start, page_end)
    WHERE page_start IS NOT NULL;

-- Index for embedding_id → chunk reverse lookup
CREATE INDEX IF NOT EXISTS idx_chunks_embedding_id
    ON chunks (embedding_id)
    WHERE embedding_id IS NOT NULL;

-- ============================================================================
-- STEP 2: chunk_entity_links — M:M mapping (SPEC-032 §3 UC-L1/L2/L4)
--
-- WHY a separate link table instead of TEXT[] in chunks:
--   - Enables efficient "which chunks for entity E?" queries with btree index
--   - Enables efficient "which entities for chunk C?" queries
--   - Supports cascade delete via foreign key (future)
--   - Normalised: no duplication of entity names across chunk rows
-- ============================================================================
CREATE TABLE IF NOT EXISTS chunk_entity_links (
    chunk_id     TEXT        NOT NULL,
    entity_name  TEXT        NOT NULL,
    workspace_id TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chunk_id, entity_name, workspace_id)
);

-- Forward: "which chunks mention entity E in workspace W?"
CREATE INDEX IF NOT EXISTS idx_cel_entity_workspace
    ON chunk_entity_links (entity_name, workspace_id);

-- Reverse: "which entities were extracted from chunk C?"
CREATE INDEX IF NOT EXISTS idx_cel_chunk_id
    ON chunk_entity_links (chunk_id);

-- Document-scoped delete: "remove all entity links for document X"
-- Resolved via JOIN chunks ON chunks.id = chunk_id WHERE chunks.document_id = X
CREATE INDEX IF NOT EXISTS idx_cel_workspace
    ON chunk_entity_links (workspace_id);

-- ============================================================================
-- STEP 3: chunk_relation_links — M:M mapping for relationships
--
-- WHY: A relation (ALICE→BOB) can appear in multiple chunks across
-- multiple documents. This table preserves full provenance.
-- ============================================================================
CREATE TABLE IF NOT EXISTS chunk_relation_links (
    chunk_id      TEXT        NOT NULL,
    source_entity TEXT        NOT NULL,
    target_entity TEXT        NOT NULL,
    workspace_id  TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chunk_id, source_entity, target_entity, workspace_id)
);

-- Forward: "which chunks contain relation (src→tgt) in workspace W?"
CREATE INDEX IF NOT EXISTS idx_crl_source_target_workspace
    ON chunk_relation_links (source_entity, target_entity, workspace_id);

-- Reverse: "which relations were extracted from chunk C?"
CREATE INDEX IF NOT EXISTS idx_crl_chunk_id
    ON chunk_relation_links (chunk_id);

-- Source-only lookup (e.g., "show all relations originating from ALICE")
CREATE INDEX IF NOT EXISTS idx_crl_source_workspace
    ON chunk_relation_links (source_entity, workspace_id);

-- Target-only lookup
CREATE INDEX IF NOT EXISTS idx_crl_target_workspace
    ON chunk_relation_links (target_entity, workspace_id);

-- ============================================================================
-- STEP 4: description_history in entities (append-only merge log)
--
-- WHY: Enables "show how entity E's description evolved across documents"
-- (UC-L4). The JSONB array is append-only per BR0020.
--
-- Schema of each element:
--   { "ts": "2026-06-29T10:00:00Z", "description": "...", "chunk_id": "..." }
-- ============================================================================
ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS description_history JSONB NOT NULL DEFAULT '[]'::jsonb;

-- GIN index for fast JSONB path queries
CREATE INDEX IF NOT EXISTS idx_entities_description_history
    ON entities USING GIN (description_history jsonb_path_ops);
