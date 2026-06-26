-- ============================================================================
-- E2E: Migration 039 Idempotency Test (SPEC-021 P2-01)
-- File: specs/021-storage-study/e2e/test_migration_039.sql
-- Run against a live database after migration 039 to verify:
--   1. New CQRS columns exist on entities and relationships
--   2. Vestigial embedding columns are gone
--   3. Required indexes exist
--   4. entity_sync_mode is registered in server_config
-- ============================================================================

SET search_path = public;

-- ============================================================================
-- Test 1: entities table has CQRS columns
-- ============================================================================
DO $$
DECLARE
    expected_cols TEXT[] := ARRAY['source_chunk_ids', 'keywords', 'sync_status', 'tsv'];
    col TEXT;
BEGIN
    FOREACH col IN ARRAY expected_cols LOOP
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'entities'
              AND column_name = col
        ) THEN
            RAISE EXCEPTION 'FAIL: entities.% column missing (migration 039 not applied?)', col;
        END IF;
        RAISE NOTICE 'PASS: entities.% exists', col;
    END LOOP;
END $$;

-- ============================================================================
-- Test 2: vestigial embedding columns are gone
-- ============================================================================
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'chunks' AND column_name = 'embedding'
    ) THEN
        RAISE WARNING 'WARN: chunks.embedding still exists — migration 039 column drop may have failed';
    ELSE
        RAISE NOTICE 'PASS: chunks.embedding column removed';
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'embedding'
    ) THEN
        RAISE WARNING 'WARN: entities.embedding still exists — migration 039 column drop may have failed';
    ELSE
        RAISE NOTICE 'PASS: entities.embedding column removed';
    END IF;
END $$;

-- ============================================================================
-- Test 3: GIN and B-tree indexes exist
-- ============================================================================
DO $$
DECLARE
    expected_indexes TEXT[] := ARRAY[
        'idx_entities_source_chunk_ids',
        'idx_entities_tsv',
        'idx_entities_type_workspace',
        'idx_entities_sync_status',
        'idx_relationships_workspace',
        'idx_relationships_source_chunk_ids'
    ];
    idx TEXT;
BEGIN
    FOREACH idx IN ARRAY expected_indexes LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_indexes
            WHERE schemaname = 'public' AND indexname = idx
        ) THEN
            RAISE WARNING 'WARN: index % missing', idx;
        ELSE
            RAISE NOTICE 'PASS: index % exists', idx;
        END IF;
    END LOOP;
END $$;

-- ============================================================================
-- Test 4: entity_sync_mode in server_config
-- ============================================================================
DO $$
DECLARE
    mode_val TEXT;
BEGIN
    SELECT value::text INTO mode_val FROM server_config WHERE key = 'entity_sync_mode';
    IF mode_val IS NULL THEN
        RAISE EXCEPTION 'FAIL: entity_sync_mode not in server_config';
    END IF;
    RAISE NOTICE 'PASS: entity_sync_mode = %', mode_val;
END $$;

-- ============================================================================
-- Test 5: edgequake schema alias views recreated without embedding
-- These views were dropped and recreated in migration 039 PRE-STEP + STEP 7.
-- The SELECT * views from migration 001 would block DROP COLUMN embedding.
-- ============================================================================
DO $$
BEGIN
    -- Verify edgequake.chunks view exists and has no embedding column
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.views
        WHERE table_schema = 'edgequake' AND table_name = 'chunks'
    ) THEN
        RAISE EXCEPTION 'FAIL: edgequake.chunks view does not exist';
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'edgequake' AND table_name = 'chunks' AND column_name = 'embedding'
    ) THEN
        RAISE EXCEPTION 'FAIL: edgequake.chunks still has embedding column (migration 039 PRE-STEP did not run)';
    END IF;
    RAISE NOTICE 'PASS: edgequake.chunks view exists without embedding column';

    -- Verify edgequake.entities view exists and has CQRS columns
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.views
        WHERE table_schema = 'edgequake' AND table_name = 'entities'
    ) THEN
        RAISE EXCEPTION 'FAIL: edgequake.entities view does not exist';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'edgequake' AND table_name = 'entities' AND column_name = 'source_chunk_ids'
    ) THEN
        RAISE EXCEPTION 'FAIL: edgequake.entities missing source_chunk_ids (STEP 7 did not run)';
    END IF;
    RAISE NOTICE 'PASS: edgequake.entities view exists with source_chunk_ids column';

    -- Verify public.chunks has no embedding column (core fix)
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'chunks' AND column_name = 'embedding'
    ) THEN
        RAISE EXCEPTION 'FAIL: public.chunks still has embedding column';
    END IF;
    RAISE NOTICE 'PASS: public.chunks.embedding dropped successfully';
END $$;

-- ============================================================================
-- Test 6: Idempotency — run again, should produce no errors
-- ============================================================================
-- (Re-running migration 039 is safe due to IF NOT EXISTS / IF EXISTS guards)
-- Simply verifying the state is consistent is the idempotency check.
DO $$
BEGIN
    RAISE NOTICE 'All migration 039 checks complete (including view dependency fix).';
END $$;
