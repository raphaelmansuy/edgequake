-- ============================================================================
-- EdgeQuake Master Migration Script
-- Version: 2.0.0
-- Date: 2025-01-28
-- Description: Apply all migrations in correct order
-- ============================================================================
--
-- USAGE:
--   cd edgequake/migrations
--   psql -U postgres -d edgequake -f scripts/apply_all_migrations.sql
--
-- OR for fresh install:
--   psql -U postgres -d edgequake -f 000_init_database.sql
--
-- ============================================================================

\echo '=============================================='
\echo 'EdgeQuake Database Migration - Starting'
\echo '=============================================='

-- Phase 0: Fresh initialization (can be run standalone for new installs)
-- \echo 'Applying 000_init_database.sql...'
-- \i ../000_init_database.sql

-- Phase 1: Core Tables
\echo 'Phase 1: Core Tables'
\echo '-------------------------------------------'

\echo 'Applying 001_add_tasks_table.sql...'
\i ../001_add_tasks_table.sql

\echo 'Applying 002_add_document_status_fields.sql...'
\i ../002_add_document_status_fields.sql

\echo 'Applying 003_add_conversation_history_table.sql...'
\i ../003_add_conversation_history_table.sql

-- Phase 2: Graph Management
\echo ''
\echo 'Phase 2: Graph Management'
\echo '-------------------------------------------'

\echo 'Applying 004_add_audit_log_table.sql...'
\i ../004_add_audit_log_table.sql

\echo 'Applying 005_add_is_manual_flags.sql...'
\i ../005_add_is_manual_flags.sql

-- Phase 3: Authentication & Multi-Tenancy
\echo ''
\echo 'Phase 3: Auth & Multi-Tenancy'
\echo '-------------------------------------------'

\echo 'Applying 006_add_auth_tables.sql...'
\i ../006_add_auth_tables.sql

\echo 'Applying 007_add_multi_tenancy_tables.sql...'
\i ../007_add_multi_tenancy_tables.sql

-- Phase 4: Row-Level Security
\echo ''
\echo 'Phase 4: Row-Level Security'
\echo '-------------------------------------------'

\echo 'Applying 008_add_rls_policies.sql...'
\i ../008_add_rls_policies.sql

-- Phase 5: Conversations
\echo ''
\echo 'Phase 5: Conversations'
\echo '-------------------------------------------'

\echo 'Applying 009_add_conversations_tables.sql...'
\i ../009_add_conversations_tables.sql

-- Phase 6: Performance & Audit
\echo ''
\echo 'Phase 6: Performance & Audit'
\echo '-------------------------------------------'

\echo 'Applying 010_tenant_performance_indexes.sql...'
\i ../010_tenant_performance_indexes.sql

\echo 'Applying 011_audit_logs_table.sql...'
\i ../011_audit_logs_table.sql

-- Phase 7: Graph Database
\echo ''
\echo 'Phase 7: Graph Database (AGE)'
\echo '-------------------------------------------'

\echo 'Applying 012_add_age_graph.sql...'
\i ../012_add_age_graph.sql

-- ============================================================================
-- Verification
-- ============================================================================

\echo ''
\echo '=============================================='
\echo 'Migration Complete - Verifying Tables'
\echo '=============================================='

-- List all tables
SELECT 
    schemaname,
    tablename,
    tableowner
FROM pg_tables 
WHERE schemaname IN ('public', 'edgequake')
ORDER BY schemaname, tablename;

-- Check extensions
\echo ''
\echo 'Installed Extensions:'
SELECT extname, extversion FROM pg_extension ORDER BY extname;

-- Check RLS status
\echo ''
\echo 'RLS Status:'
SELECT 
    schemaname,
    tablename,
    rowsecurity
FROM pg_tables 
WHERE schemaname = 'public' 
  AND rowsecurity = true
ORDER BY tablename;

\echo ''
\echo '=============================================='
\echo 'EdgeQuake Database Migration - Complete!'
\echo 'Version: 2.0.0'
\echo '=============================================='
