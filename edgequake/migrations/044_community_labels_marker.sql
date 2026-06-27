-- Migration 044: Schema version marker (SPEC-023 I6)
--
-- Community label backfill runs via graph startup hook
-- (community_persist.rs) after sqlx marker 044 is recorded.

DO $$
BEGIN
    RAISE NOTICE 'Migration 044 marker recorded. Community backfill runs at graph startup';
END $$;
