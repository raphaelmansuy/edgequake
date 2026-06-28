-- Migration 062: auth/mod.rs routes identity through identity_storage (SPEC-027 phase 51)

DO $$
BEGIN
    RAISE NOTICE 'Migration 062 marker recorded. handlers/auth isolated from auth_kv_store';
END $$;
