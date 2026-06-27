-- SPEC-023 Migration 044 — community labels marker (SSOT)
--
-- Used by migration_bootstrap.rs (startup, after sqlx marker 044).
-- Graph community_id backfill is performed in Rust (community_persist.rs).

DO $$
BEGIN
    RAISE NOTICE 'Migration 044 apply — community labels backfill delegated to graph startup hook';
END $$;
