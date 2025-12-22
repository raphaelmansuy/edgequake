-- Phase 3 Combined Migration Script
-- Run all Phase 3 migrations in order
-- Usage: psql -d edgequake -f phase3_apply_all.sql

\echo 'Starting Phase 3 migrations...'

\echo 'Applying 006_add_auth_tables.sql...'
\i 006_add_auth_tables.sql

\echo 'Applying 007_add_multi_tenancy_tables.sql...'
\i 007_add_multi_tenancy_tables.sql

\echo 'Phase 3 migrations complete!'

-- Verify tables exist
\echo 'Verifying auth tables...'
SELECT 
    'users' AS table_name, 
    COUNT(*) AS row_count 
FROM users
UNION ALL
SELECT 
    'api_keys' AS table_name, 
    COUNT(*) AS row_count 
FROM api_keys
UNION ALL
SELECT 
    'refresh_tokens' AS table_name, 
    COUNT(*) AS row_count 
FROM refresh_tokens
UNION ALL
SELECT 
    'tenants' AS table_name, 
    COUNT(*) AS row_count 
FROM tenants
UNION ALL
SELECT 
    'workspaces' AS table_name, 
    COUNT(*) AS row_count 
FROM workspaces
UNION ALL
SELECT 
    'memberships' AS table_name, 
    COUNT(*) AS row_count 
FROM memberships;

\echo 'Phase 3 verification complete!'
