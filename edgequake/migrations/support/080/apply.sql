-- SPEC-042-E E-01 — halfvec embedding column conversion (SSOT)
--
-- Invoked by migration_bootstrap when EDGEQUAKE_VECTOR_STORAGE=halfvec.
-- Idempotent: skips tables already using halfvec.

DO $$
DECLARE
    tbl record;
    udt text;
    dim int;
    idx record;
    converted int := 0;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
        RAISE NOTICE 'Migration 080 apply — pgvector not installed, skipping';
        RETURN;
    END IF;

    FOR tbl IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq\_%\_vectors' ESCAPE '\'
    LOOP
        SELECT c.udt_name
        INTO udt
        FROM information_schema.columns c
        WHERE c.table_schema = 'public'
          AND c.table_name = tbl.tablename
          AND c.column_name = 'embedding';

        IF NOT FOUND OR udt = 'halfvec' THEN
            CONTINUE;
        END IF;

        IF udt <> 'vector' THEN
            RAISE NOTICE 'Migration 080 apply — % embedding type % (skip)', tbl.tablename, udt;
            CONTINUE;
        END IF;

        SELECT (regexp_match(format_type(a.atttypid, a.atttypmod), 'vector\((\d+)\)'))[1]::int
        INTO dim
        FROM pg_attribute a
        JOIN pg_class c ON c.oid = a.attrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relname = tbl.tablename
          AND a.attname = 'embedding'
          AND NOT a.attisdropped;

        IF dim IS NULL THEN
            RAISE EXCEPTION 'Migration 080 apply — cannot resolve vector dimension for %', tbl.tablename;
        END IF;

        IF dim > 4000 THEN
            RAISE NOTICE 'Migration 080 apply — % dim=% exceeds halfvec HNSW max (4000); skipping (#275)', tbl.tablename, dim;
            CONTINUE;
        END IF;

        RAISE NOTICE 'Migration 080 apply — converting %.embedding vector(%) → halfvec(%)', tbl.tablename, dim, dim;

        FOR idx IN
            SELECT indexname
            FROM pg_indexes
            WHERE schemaname = 'public'
              AND tablename = tbl.tablename
              AND indexdef ILIKE '%embedding%'
              AND (indexdef ILIKE '%USING hnsw%' OR indexdef ILIKE '%USING ivfflat%')
        LOOP
            EXECUTE format('DROP INDEX IF EXISTS %I', idx.indexname);
        END LOOP;

        EXECUTE format(
            'ALTER TABLE %I ALTER COLUMN embedding TYPE halfvec(%s) USING embedding::halfvec(%s)',
            tbl.tablename, dim, dim
        );

        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I USING hnsw (embedding halfvec_cosine_ops)',
            tbl.tablename || '_embedding_halfvec_hnsw_idx',
            tbl.tablename
        );

        converted := converted + 1;
    END LOOP;

    RAISE NOTICE 'Migration 080 apply complete — converted % vector tables to halfvec', converted;
END $$;
