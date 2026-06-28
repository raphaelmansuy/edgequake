-- Migration 046: Schema version marker — graph tenant isolation + query perf indexes
--
-- Idempotent index DDL runs via migration_bootstrap
-- (migrations/support/046/apply.sql), matching the filter-first SQL paths in
-- edgequake-storage (pg_get_popular_nodes_with_degree, get_edges_for_node_set).

DO $$
BEGIN
    RAISE NOTICE 'Migration 046 marker recorded. Graph isolation perf indexes run via migration_bootstrap';
END $$;
