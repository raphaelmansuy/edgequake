-- Migration 063: auth_kv_store service-layer-only SSOT (SPEC-027 phase 52)

DO $$
BEGIN
    RAISE NOTICE 'Migration 063 marker recorded. auth_kv_store reachable only via identity/session storage';
END $$;
