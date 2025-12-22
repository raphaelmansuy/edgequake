-- EdgeQuake Phase 1 Migrations (v1.1.0)
-- Complete migration file for Phase 1 enhancements
-- Date: 2025-12-22

-- Apply all migrations in order
\i 001_add_tasks_table.sql
\i 002_add_document_status_fields.sql
\i 003_add_conversation_history_table.sql

-- Success message
DO $$ BEGIN
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'EdgeQuake Phase 1 migrations completed!';
    RAISE NOTICE 'Version: 1.1.0';
    RAISE NOTICE '==============================================';
END $$;
