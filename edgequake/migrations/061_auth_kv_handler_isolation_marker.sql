-- Migration 061: Handlers route identity through identity_storage (SPEC-027 phase 50)

DO $$
BEGIN
    RAISE NOTICE 'Migration 061 marker recorded. user_management isolated from auth_kv_store';
END $$;
