-- Migration 059: PG-only vs KV test-harness branch SSOT (SPEC-027 phase 48 DRY)

DO $$
BEGIN
    RAISE NOTICE 'Migration 059 marker recorded. Auth storage uses pg_primary/else branches';
END $$;
