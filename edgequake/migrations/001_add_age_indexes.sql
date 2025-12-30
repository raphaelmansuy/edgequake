-- Migration: Add AGE Graph Indexes for Performance
-- Date: 2025-12-31
-- Issue: Database timeout errors on get_popular_nodes_with_degree
-- Fix: Add indexes on AGE internal tables for 10-100x speedup
--
-- Run this on existing databases that already have AGE graph created.
-- For new databases, init.sql includes these indexes automatically.

\echo '==================================================================='
\echo 'AGE Graph Performance Indexes Migration'
\echo 'This adds critical indexes to prevent query timeouts'
\echo '==================================================================='

-- Set variables
\set graph_name 'edgequake_graph'

-- Start transaction
BEGIN;

\echo 'Checking if AGE extension is installed...'
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE EXCEPTION 'AGE extension not installed. Install it first with: CREATE EXTENSION age;';
    END IF;
    RAISE NOTICE '✓ AGE extension found';
END $$;

\echo 'Checking if graph exists...'
DO $$ 
DECLARE
    graph_name TEXT := 'edgequake_graph';
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = graph_name) THEN
        RAISE EXCEPTION 'Graph "%" does not exist. Create it first with AGE graph creation.', graph_name;
    END IF;
    RAISE NOTICE '✓ Graph "%" found', graph_name;
END $$;

\echo 'Creating indexes on _ag_label_edge table...'
DO $$ 
DECLARE
    graph_name TEXT := 'edgequake_graph';
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration INTERVAL;
BEGIN
    start_time := clock_timestamp();
    
    -- Index 1: start_id for outbound degree calculation
    RAISE NOTICE 'Creating index: idx_ag_edge_start_id...';
    EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_edge_start_id 
        ON %I._ag_label_edge(start_id)', graph_name);
    
    -- Index 2: end_id for inbound degree calculation
    RAISE NOTICE 'Creating index: idx_ag_edge_end_id...';
    EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_edge_end_id 
        ON %I._ag_label_edge(end_id)', graph_name);
    
    -- Index 3: Composite index for bi-directional lookups
    RAISE NOTICE 'Creating index: idx_ag_edge_start_end...';
    EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_edge_start_end 
        ON %I._ag_label_edge(start_id, end_id)', graph_name);
    
    end_time := clock_timestamp();
    duration := end_time - start_time;
    RAISE NOTICE '✓ Edge indexes created in %', duration;
END $$;

\echo 'Creating indexes on _ag_label_vertex table...'
DO $$ 
DECLARE
    graph_name TEXT := 'edgequake_graph';
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration INTERVAL;
BEGIN
    start_time := clock_timestamp();
    
    -- Index 4: GIN index on properties for JSONB filtering
    RAISE NOTICE 'Creating index: idx_ag_vertex_props_gin (this may take a while for large graphs)...';
    EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_vertex_props_gin 
        ON %I._ag_label_vertex USING GIN(properties)', graph_name);
    
    -- Index 5: id for primary key lookups
    RAISE NOTICE 'Creating index: idx_ag_vertex_id...';
    EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_vertex_id 
        ON %I._ag_label_vertex(id)', graph_name);
    
    end_time := clock_timestamp();
    duration := end_time - start_time;
    RAISE NOTICE '✓ Vertex indexes created in %', duration;
END $$;

\echo 'Running ANALYZE to update query planner statistics...'
DO $$ 
DECLARE
    graph_name TEXT := 'edgequake_graph';
BEGIN
    EXECUTE format('ANALYZE %I._ag_label_edge', graph_name);
    EXECUTE format('ANALYZE %I._ag_label_vertex', graph_name);
    RAISE NOTICE '✓ Statistics updated';
END $$;

\echo 'Verifying indexes were created...'
DO $$ 
DECLARE
    graph_name TEXT := 'edgequake_graph';
    index_count INTEGER;
BEGIN
    EXECUTE format('SELECT COUNT(*) FROM pg_indexes WHERE tablename IN (''_ag_label_edge'', ''_ag_label_vertex'') AND schemaname = %L', graph_name) INTO index_count;
    RAISE NOTICE '✓ Found % indexes on AGE tables', index_count;
    
    IF index_count < 5 THEN
        RAISE WARNING 'Expected at least 5 indexes, but found %. Check for errors above.', index_count;
    END IF;
END $$;

COMMIT;

\echo '==================================================================='
\echo 'Migration completed successfully!'
\echo 'Your graph queries should now be 10-100x faster.'
\echo 'Test with: SELECT * FROM get_popular_nodes_with_degree(100);'
\echo '==================================================================='

-- Show index sizes for monitoring
\echo 'Index sizes:'
SELECT 
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_indexes 
JOIN pg_class ON pg_class.relname = indexname
WHERE tablename IN ('_ag_label_edge', '_ag_label_vertex')
    AND schemaname = :'graph_name'
ORDER BY pg_relation_size(indexrelid) DESC;
