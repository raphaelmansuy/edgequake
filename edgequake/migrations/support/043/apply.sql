-- SPEC-022 Migration 043 — Apache AGE extension upgrade (SSOT)
--
-- Used by migration_bootstrap.rs (startup, after sqlx marker 043).
--
-- WHY: Docker ships pgvector 0.8.3 + AGE 1.6.0 (Dockerfile.postgres); existing volumes may have
-- an older extversion. ALTER EXTENSION UPDATE aligns the catalog with the
-- installed shared library (required for parameterized Cypher / prepared statements).

DO $$
DECLARE
    ext_version text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Apache AGE not installed — skipping migration 043 apply';
        RETURN;
    END IF;

    SELECT extversion INTO ext_version FROM pg_extension WHERE extname = 'age';
    RAISE NOTICE 'Migration 043 apply — AGE extversion before: %', ext_version;

    BEGIN
        ALTER EXTENSION age UPDATE;
        SELECT extversion INTO ext_version FROM pg_extension WHERE extname = 'age';
        RAISE NOTICE 'AGE extversion after ALTER EXTENSION UPDATE: %', ext_version;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'ALTER EXTENSION age UPDATE skipped (already latest or library mismatch): %', SQLERRM;
    END;

    RAISE NOTICE 'Migration 043 apply complete';
END $$;
