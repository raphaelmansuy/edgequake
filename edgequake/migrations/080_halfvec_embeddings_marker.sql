-- Migration 080: halfvec embeddings marker (SPEC-042-E E-01)
--
-- Column conversion + HNSW reindex runs via migration_bootstrap
-- (migrations/support/080/apply.sql) when EDGEQUAKE_VECTOR_STORAGE=halfvec.

DO $$
BEGIN
    RAISE NOTICE 'Migration 080 marker recorded. halfvec conversion runs via migration_bootstrap when EDGEQUAKE_VECTOR_STORAGE=halfvec';
END $$;
