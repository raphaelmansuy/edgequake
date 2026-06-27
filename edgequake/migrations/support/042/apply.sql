-- SPEC-022 Migration 042 — pgvector extension upgrade + ANN index rebuild (SSOT)
--
-- Used by:
--   migration_bootstrap.rs (startup, after sqlx marker 042)
--
-- WHY: pgvector minor upgrades (0.7.x → 0.8.x) require ALTER EXTENSION UPDATE
-- and HNSW/IVFFlat index rebuild for iterative-scan GUCs to work reliably.

DO $$
DECLARE
    ext_version text;
    tbl record;
    idx record;
    reindexed int := 0;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
        RAISE NOTICE 'pgvector not installed — skipping migration 042 apply';
        RETURN;
    END IF;

    SELECT extversion INTO ext_version FROM pg_extension WHERE extname = 'vector';
    RAISE NOTICE 'Migration 042 apply — pgvector extversion before: %', ext_version;

    BEGIN
        ALTER EXTENSION vector UPDATE;
        SELECT extversion INTO ext_version FROM pg_extension WHERE extname = 'vector';
        RAISE NOTICE 'pgvector extversion after ALTER EXTENSION UPDATE: %', ext_version;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'ALTER EXTENSION vector UPDATE skipped (already latest or library mismatch): %', SQLERRM;
    END;

    FOR tbl IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq\_%\_vectors' ESCAPE '\'
    LOOP
        FOR idx IN
            SELECT indexname, indexdef
            FROM pg_indexes
            WHERE schemaname = 'public'
              AND tablename = tbl.tablename
              AND indexdef ILIKE '%embedding%'
              AND (indexdef ILIKE '%USING hnsw%' OR indexdef ILIKE '%USING ivfflat%')
        LOOP
            BEGIN
                EXECUTE format('REINDEX INDEX %I', idx.indexname);
                reindexed := reindexed + 1;
                RAISE NOTICE '  ✓ reindexed % on %', idx.indexname, tbl.tablename;
            EXCEPTION WHEN OTHERS THEN
                RAISE NOTICE '  ✗ reindex % on % failed: %', idx.indexname, tbl.tablename, SQLERRM;
            END;
        END LOOP;
    END LOOP;

    RAISE NOTICE 'Migration 042 apply complete — reindexed % vector ANN indexes', reindexed;
END $$;
