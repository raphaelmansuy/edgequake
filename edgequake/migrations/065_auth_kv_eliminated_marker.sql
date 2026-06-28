-- Migration 065: authentication eliminated from KV (SPEC-027 phase 55)

DO $$
BEGIN
    RAISE NOTICE 'Migration 065 marker recorded. Auth uses PostgreSQL or AuthMemoryStore only';
END $$;
