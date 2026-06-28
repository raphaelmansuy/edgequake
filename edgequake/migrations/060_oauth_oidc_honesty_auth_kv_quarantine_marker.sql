-- Migration 060: OAuth2/OIDC honesty + auth_kv_store quarantine marker (SPEC-027 phase 49)

DO $$
BEGIN
    RAISE NOTICE 'Migration 060 marker recorded. OAuth2/OIDC not builtin; KV auth test-harness only';
END $$;
