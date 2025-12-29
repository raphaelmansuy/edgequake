-- Migration: Add tenant isolation performance indexes
-- Version: V003
-- Description: Optimize tenant-filtered queries with strategic indexes
-- Created: 2024-12-29

-- ============================================================================
-- VECTOR STORAGE INDEXES
-- ============================================================================

-- Composite index for tenant + workspace filtering on chunks table
-- This enables fast tenant-scoped vector searches
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_tenant_workspace 
ON chunks(tenant_id, workspace_id) 
WHERE tenant_id IS NOT NULL;

-- BRIN index for time-series queries (efficient for large tables)
-- Useful for "recent documents" queries within a tenant
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_created_at_brin 
ON chunks USING BRIN(created_at, tenant_id)
WITH (pages_per_range = 128);

-- Index for tenant-only filtering (when workspace_id is NULL)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_tenant_only
ON chunks(tenant_id)
WHERE tenant_id IS NOT NULL AND workspace_id IS NULL;

-- Covering index for chunk metadata retrieval without table access
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_tenant_metadata
ON chunks(tenant_id, workspace_id, document_id, chunk_id)
INCLUDE (created_at, metadata);

-- ============================================================================
-- GRAPH STORAGE INDEXES  
-- ============================================================================

-- JSONB GIN indexes for tenant/workspace properties on entities
-- Enables fast filtering: WHERE properties->>'tenant_id' = 'xxx'
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_tenant_id_gin
ON entities USING GIN ((properties->>'tenant_id') gin_trgm_ops);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_workspace_id_gin
ON entities USING GIN ((properties->>'workspace_id') gin_trgm_ops);

-- Composite B-tree index for exact tenant + workspace lookups
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_tenant_workspace
ON entities((properties->>'tenant_id'), (properties->>'workspace_id'));

-- Index for entity type filtering within tenant
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_tenant_type
ON entities((properties->>'tenant_id'), entity_type);

-- Similar indexes for edges (relationships)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edges_tenant_id_gin
ON edges USING GIN ((properties->>'tenant_id') gin_trgm_ops);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edges_workspace_id_gin
ON edges USING GIN ((properties->>'workspace_id') gin_trgm_ops);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edges_tenant_workspace
ON edges((properties->>'tenant_id'), (properties->>'workspace_id'));

-- Index for relationship queries within tenant
-- Example: Find all edges of specific type for a tenant
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edges_tenant_relationship
ON edges((properties->>'tenant_id'), relationship_type);

-- ============================================================================
-- DOCUMENT METADATA INDEXES
-- ============================================================================

-- Primary tenant + workspace composite index
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_documents_tenant_workspace 
ON documents(tenant_id, workspace_id)
WHERE tenant_id IS NOT NULL;

-- Index for document status filtering within tenant
-- Useful for "show all processing documents for tenant X"
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_documents_tenant_status
ON documents(tenant_id, status)
INCLUDE (title, created_at, updated_at);

-- Index for full-text search within tenant scope
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_documents_tenant_title_search
ON documents USING GIN (to_tsvector('english', title))
WHERE tenant_id IS NOT NULL;

-- ============================================================================
-- AUDIT LOG INDEXES (if audit_logs table exists)
-- ============================================================================

-- Composite index for tenant + timestamp queries
-- Enables fast "show audit log for tenant X in last 24 hours"
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_logs_tenant_timestamp
ON audit_logs(tenant_id, timestamp DESC)
WHERE tenant_id IS NOT NULL;

-- Index for security event queries
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_logs_event_type_timestamp
ON audit_logs(event_type, timestamp DESC);

-- Index for failed/blocked events
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_logs_result_timestamp
ON audit_logs(result, timestamp DESC)
WHERE result IN ('Failure', 'Blocked');

-- ============================================================================
-- PERFORMANCE OPTIMIZATION HINTS
-- ============================================================================

-- Update table statistics for better query planning
ANALYZE chunks;
ANALYZE entities;
ANALYZE edges;
ANALYZE documents;
ANALYZE audit_logs;

-- ============================================================================
-- INDEX MONITORING QUERIES (for verification)
-- ============================================================================

-- Query to verify index usage (run after migration)
-- SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
-- FROM pg_stat_user_indexes
-- WHERE indexname LIKE 'idx_%_tenant%'
-- ORDER BY idx_scan DESC;

-- Query to check index bloat
-- SELECT schemaname, tablename, indexname, 
--        pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
-- FROM pg_stat_user_indexes
-- WHERE indexname LIKE 'idx_%_tenant%'
-- ORDER BY pg_relation_size(indexrelid) DESC;

-- ============================================================================
-- ROLLBACK SCRIPT (if needed)
-- ============================================================================

-- DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_tenant_workspace;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_created_at_brin;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_tenant_only;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_tenant_metadata;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_entities_tenant_id_gin;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_entities_workspace_id_gin;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_entities_tenant_workspace;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_entities_tenant_type;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_edges_tenant_id_gin;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_edges_workspace_id_gin;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_edges_tenant_workspace;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_edges_tenant_relationship;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_documents_tenant_workspace;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_documents_tenant_status;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_documents_tenant_title_search;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_audit_logs_tenant_timestamp;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_audit_logs_event_type_timestamp;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_audit_logs_result_timestamp;
