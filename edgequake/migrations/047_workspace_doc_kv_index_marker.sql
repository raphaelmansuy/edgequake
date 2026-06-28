-- Migration 047: Schema version marker — workspace document KV index (wsdoc:)
--
-- Idempotent backfill of wsdoc:{workspace_id}:{document_id} pointer keys runs via
-- migration_bootstrap (migrations/support/047/apply.sql).

DO $$
BEGIN
    RAISE NOTICE 'Migration 047 marker recorded. Workspace doc KV index backfill runs via migration_bootstrap';
END $$;
