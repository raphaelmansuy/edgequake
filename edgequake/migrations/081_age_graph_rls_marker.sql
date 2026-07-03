-- Migration 081: AGE graph RLS marker (SPEC-042-E E-02)
--
-- ENABLE ROW LEVEL SECURITY on AGE label tables runs via migration_bootstrap
-- (migrations/support/081/apply.sql) when EDGEQUAKE_AGE_RLS=true and AGE >= 1.7.0.

DO $$
BEGIN
    RAISE NOTICE 'Migration 081 marker recorded. AGE RLS policies run via migration_bootstrap when EDGEQUAKE_AGE_RLS=true';
END $$;
