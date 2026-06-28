-- Migration 058: Schema version marker — KV identity mirror hard-disabled when PG pool (SPEC-027 phase 47)
--
-- EDGEQUAKE_KV_IDENTITY_MIRROR is ignored at runtime when PostgreSQL pool + pg_identity_ssot.

DO $$
BEGIN
    RAISE NOTICE 'Migration 058 marker recorded. KV identity mirror ignored when PG pool exists';
END $$;
