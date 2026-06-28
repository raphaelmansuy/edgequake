-- Migration 057: Schema version marker — KV identity mirror deprecated (SPEC-027 phase 46)
--
-- EDGEQUAKE_KV_IDENTITY_MIRROR is legacy; PostgreSQL is auth SSOT when pool exists.

DO $$
BEGIN
    RAISE NOTICE 'Migration 057 marker recorded. KV identity mirror deprecated';
END $$;
