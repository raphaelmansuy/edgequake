-- EdgeQuake Phase 2 Migrations (v1.2.0)
-- Complete migration file for Phase 2 graph management enhancements
-- Date: 2025-12-22

-- Apply all migrations in order
\i 004_add_audit_log_table.sql
\i 005_add_is_manual_flags.sql

-- Success message
DO $$ BEGIN
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'EdgeQuake Phase 2 migrations completed!';
    RAISE NOTICE 'Version: 1.2.0';
    RAISE NOTICE '==============================================';
END $$;
