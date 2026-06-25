-- ============================================================================
-- Migration 039: CQRS entities schema correction + drop vestigial columns
-- Version: 1.0.0
-- Date: 2026-06-25
-- Author: EdgeQuake Team (SPEC-021)
--
-- PURPOSE:
--   1. Correct the `entities` and `relationships` tables to match the actual
--      pipeline data model (they were originally defined but never populated;
--      now they serve as the CQRS read model for analytics/FTS/JOINs).
--   2. Drop the vestigial `embedding` columns from `chunks` and `entities`
--      (always NULL — the pipeline stores embeddings in eq_*_vectors tables).
--   3. Add GIN indexes for fast deletion cascade and FTS.
--   4. Register entity_sync_mode in server_config for feature-gated dual-write.
--
-- ASCENDING COMPATIBILITY:
--   * All changes are ADDITIVE (new columns with defaults) EXCEPT the column drops.
--   * Column drops are safe: chunks.embedding and entities.embedding are always NULL
--     in production — verified by SPEC-021 analysis of pipeline code.
--   * entity_sync_mode defaults to "disabled": no dual-write until explicitly enabled.
--   * IDEMPOTENT: safe to re-run (all DDL uses IF NOT EXISTS / IF EXISTS).
--
-- ROLLBACK:
--   DROP INDEX IF EXISTS idx_entities_source_chunk_ids;
--   DROP INDEX IF EXISTS idx_entities_tsv;
--   DROP INDEX IF EXISTS idx_entities_type_workspace;
--   DROP INDEX IF EXISTS idx_entities_sync_status;
--   DROP INDEX IF EXISTS idx_relationships_workspace;
--   DROP INDEX IF EXISTS idx_relationships_source_chunk_ids;
--   ALTER TABLE entities DROP COLUMN IF EXISTS source_chunk_ids;
--   ALTER TABLE entities DROP COLUMN IF EXISTS sync_status;
--   ALTER TABLE entities DROP COLUMN IF EXISTS tsv;
--   ALTER TABLE entities DROP COLUMN IF EXISTS keywords;
--   ALTER TABLE relationships DROP COLUMN IF EXISTS relation_type;
--   ALTER TABLE relationships DROP COLUMN IF EXISTS description;
--   ALTER TABLE relationships DROP COLUMN IF EXISTS keywords;
--   ALTER TABLE relationships DROP COLUMN IF EXISTS weight;
--   ALTER TABLE relationships DROP COLUMN IF EXISTS source_chunk_ids;
--   ALTER TABLE relationships DROP COLUMN IF EXISTS sync_status;
--   DELETE FROM server_config WHERE key = 'entity_sync_mode';
-- ============================================================================

SET search_path = public;

-- ============================================================================
-- PRE-STEP: Drop edgequake schema alias views that reference embedding columns
-- WHY: These SELECT * views were created in migration 001 and refreshed in 031.
-- PostgreSQL resolves SELECT * at view-creation time, recording individual column
-- references. They must be dropped before we can drop the underlying columns.
-- They are recreated below (STEP 7) with explicit column lists that exclude
-- embedding and include the new CQRS columns.
-- Only edgequake.chunks and edgequake.entities reference the embedding columns;
-- the tenant_entity_stats view uses COUNT(*) and is unaffected.
-- ============================================================================
DROP VIEW IF EXISTS edgequake.chunks;
DROP VIEW IF EXISTS edgequake.entities;

-- ============================================================================
-- STEP 1: Drop vestigial embedding columns
-- These columns are ALWAYS NULL in production: the pipeline writes embeddings
-- to eq_*_vectors tables, NOT to these relational columns.
-- Verified: no INSERT/UPDATE to entities.embedding or chunks.embedding anywhere
-- in the active pipeline codebase (grep confirms - SPEC-021 R-DRY-02).
-- ============================================================================
DO $$
BEGIN
    -- Drop HNSW index on chunks.embedding if it exists
    BEGIN
        DROP INDEX IF EXISTS idx_chunks_embedding;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'idx_chunks_embedding not found, skipping';
    END;

    -- Drop HNSW index on entities.embedding if it exists
    BEGIN
        DROP INDEX IF EXISTS idx_entities_embedding;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'idx_entities_embedding not found, skipping';
    END;
END $$;

ALTER TABLE chunks   DROP COLUMN IF EXISTS embedding;
ALTER TABLE entities DROP COLUMN IF EXISTS embedding;

-- ============================================================================
-- STEP 2: Add CQRS columns to entities table
-- These enable the entities table to serve as an analytics read model.
-- ============================================================================

-- source_chunk_ids: TEXT[] of contributing chunk KV keys (e.g. "{doc_id}-chunk-0")
-- WHY TEXT[]: The pipeline uses string chunk IDs (not UUID), matching the KV store keys.
-- WHY GIN: Enables fast "which entities came from document X?" queries (deletion cascade).
ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS source_chunk_ids TEXT[]   NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS keywords         TEXT[]   NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS sync_status      VARCHAR(20) NOT NULL DEFAULT 'unsynced';

-- stored tsvector for FTS — generated, never stale
-- WHY: More efficient than GIN on agtype_to_json expression (which is how AGE indexes FTS).
-- REQUIRES PostgreSQL 12+ (GENERATED ALWAYS AS STORED)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'entities'
          AND column_name = 'tsv'
    ) THEN
        ALTER TABLE entities
            ADD COLUMN tsv TSVECTOR
                GENERATED ALWAYS AS (
                    to_tsvector('english',
                        coalesce(name, '') || ' ' ||
                        coalesce(entity_type, '') || ' ' ||
                        coalesce(description, '')
                    )
                ) STORED;
        RAISE NOTICE 'Added entities.tsv generated column';
    ELSE
        RAISE NOTICE 'entities.tsv already exists, skipping';
    END IF;
END $$;

-- ============================================================================
-- STEP 3: Add CQRS columns to relationships table
-- ============================================================================
ALTER TABLE relationships
    ADD COLUMN IF NOT EXISTS relation_type    TEXT          DEFAULT 'RELATED_TO',
    ADD COLUMN IF NOT EXISTS description      TEXT,
    ADD COLUMN IF NOT EXISTS keywords         TEXT[]        NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS weight           FLOAT         NOT NULL DEFAULT 0.5,
    ADD COLUMN IF NOT EXISTS source_chunk_ids TEXT[]        NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS tenant_id        UUID,
    ADD COLUMN IF NOT EXISTS workspace_id     UUID,
    ADD COLUMN IF NOT EXISTS sync_status      VARCHAR(20)   NOT NULL DEFAULT 'unsynced';

-- ============================================================================
-- STEP 4: Performance indexes for CQRS read model
-- ============================================================================

-- GIN on source_chunk_ids for fast deletion cascade scan
-- WHY: The delete_document() path needs to find all entities sourced by a document.
-- Without this GIN index, it scans all entities. With it: O(log N + K).
CREATE INDEX IF NOT EXISTS idx_entities_source_chunk_ids
    ON entities USING GIN (source_chunk_ids);

-- GIN on tsv for full-text search
-- WHY: Replaces the AGE expression GIN (agtype_to_json(properties)->>'node_id')
-- with a proper stored tsvector that PostgreSQL can optimize efficiently.
CREATE INDEX IF NOT EXISTS idx_entities_tsv
    ON entities USING GIN (tsv);

-- B-tree for analytics: entity counts by type and workspace
CREATE INDEX IF NOT EXISTS idx_entities_type_workspace
    ON entities (entity_type, tenant_id, workspace_id)
    WHERE workspace_id IS NOT NULL;

-- Partial index to find unsynced entities during repair
CREATE INDEX IF NOT EXISTS idx_entities_sync_status
    ON entities (sync_status)
    WHERE sync_status != 'synced';

-- Relationships: workspace-scoped analytics queries
CREATE INDEX IF NOT EXISTS idx_relationships_workspace
    ON relationships (tenant_id, workspace_id)
    WHERE workspace_id IS NOT NULL;

-- Relationships: GIN on source_chunk_ids for deletion cascade
CREATE INDEX IF NOT EXISTS idx_relationships_source_chunk_ids
    ON relationships USING GIN (source_chunk_ids);

-- ============================================================================
-- STEP 5: Register entity_sync_mode in server_config
-- Controls dual-write behavior (SPEC-021 ascending compatibility):
--   "disabled"   → no dual-write (default, old behaviour preserved)
--   "dual_write" → new writes go to both AGE and entities table
--   "full"       → dual_write + backfill complete (set by migration 040 apply.sql)
-- ============================================================================
INSERT INTO server_config (key, value)
VALUES ('entity_sync_mode', '"disabled"')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- STEP 6: Table documentation comments
-- ============================================================================
COMMENT ON TABLE entities IS 'CQRS read model for entity analytics, FTS, and JOIN queries. Primary source for graph traversal: Apache AGE (see SPEC-021). Populated by dual-write in KnowledgeGraphMerger when entity_sync_mode != disabled. Backfilled from AGE by migration 040 apply script.';

COMMENT ON TABLE relationships IS 'CQRS read model for relationship analytics. Primary source for graph traversal: Apache AGE EDGE (see SPEC-021).';

COMMENT ON COLUMN entities.source_chunk_ids IS 'TEXT[] of KV chunk keys that contributed to this entity. Format: "{doc_id}-chunk-{n}". Used for cascade delete. Corresponds to AGE Node.source_ids property.';

COMMENT ON COLUMN entities.sync_status IS 'Sync state: unsynced | synced | pending_repair. Set to synced by dual-write in KnowledgeGraphMerger. Set to unsynced on insert (pre-backfill).';

-- ============================================================================
-- STEP 7: Recreate edgequake schema alias views without embedding columns
-- WHY: Explicit column lists prevent future drift when new columns are added.
-- These views expose the CQRS columns (source_chunk_ids, keywords, sync_status,
-- tsv) that were added in STEP 2, and omit the dropped embedding column.
-- ============================================================================
CREATE OR REPLACE VIEW edgequake.chunks AS
  SELECT
    id,
    document_id,
    tenant_id,
    workspace_id,
    content,
    chunk_index,
    start_offset,
    end_offset,
    token_count,
    metadata,
    created_at
  FROM public.chunks;

CREATE OR REPLACE VIEW edgequake.entities AS
  SELECT
    id,
    tenant_id,
    workspace_id,
    name,
    entity_type,
    description,
    source_ids,
    source_chunk_ids,
    keywords,
    sync_status,
    tsv,
    is_manual,
    manual_created_at,
    manual_created_by,
    last_manual_edit_at,
    last_manual_edit_by,
    metadata,
    created_at,
    updated_at
  FROM public.entities;

-- NOTE: RAISE NOTICE is only valid inside PL/pgSQL blocks.
DO $$
BEGIN
    RAISE NOTICE 'Migration 039 complete: entities/relationships CQRS schema prepared.';
    RAISE NOTICE 'entity_sync_mode = disabled (dual-write inactive until manually enabled).';
    RAISE NOTICE 'Run migration 040 apply script to backfill entities from AGE graph.';
END $$;
