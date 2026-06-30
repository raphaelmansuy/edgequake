-- SPEC-023 Migration 045 — native Postgres FTS on vector chunk content (SSOT)
--
-- Adds GIN-backed `content_tsv` (generated from metadata->>'content') to all
-- `eq_*_vectors` tables for ts_rank_cd sparse retrieval at query time.

DO $$
DECLARE
    tbl record;
    added int := 0;
BEGIN
    FOR tbl IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq\_%\_vectors' ESCAPE '\'
    LOOP
        BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = tbl.tablename
                  AND column_name = 'content_tsv'
            ) THEN
                EXECUTE format(
                    'ALTER TABLE %I ADD COLUMN content_tsv TSVECTOR
                     GENERATED ALWAYS AS (
                         to_tsvector(''english'', coalesce(metadata->>''content'', ''''))
                     ) STORED',
                    tbl.tablename
                );
                added := added + 1;
                RAISE NOTICE '  ✓ added content_tsv on %', tbl.tablename;
            END IF;

            -- SPEC-034 IMP-05: Do NOT create the idx_%s_content_tsv duplicate.
            -- WHY: The canonical index eq_%s_vectors_content_tsv_idx is created by
            --      the vector DDL bootstrap (ddl.rs ensure_content_fts). The idx_ form
            --      was a duplicate with 0 additional query coverage at 1.9 MB each.
            --      Only add the column here; index creation is owned by the DDL layer.
            NULL; -- index creation intentionally removed
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ content_tsv on % failed: %', tbl.tablename, SQLERRM;
        END;
    END LOOP;

    RAISE NOTICE 'Migration 045 apply complete — content_tsv added on % vector table(s)', added;
END $$;
