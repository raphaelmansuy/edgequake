-- Migration 064: builtin OIDC authorization-code flow (SPEC-027 phase 54)

DO $$
BEGIN
    RAISE NOTICE 'Migration 064 marker recorded. OIDC routes active when EDGEQUAKE_OIDC_ENABLED=true';
END $$;
