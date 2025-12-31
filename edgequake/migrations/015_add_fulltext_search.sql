-- Add full-text search index for node_id property
-- Migration: 015_add_fulltext_search.sql
-- Purpose: Enable fuzzy search and autocomplete for entity names
-- Performance: Enables ts_rank scoring and @@ operator matching

-- NOTE: Using non-CONCURRENT index creation for compatibility with migration transactions
-- For production with large tables, consider running CONCURRENTLY outside migrations

-- Create pg_trgm extension if not exists (needed for trigram similarity)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Create indexes on graph vertex tables
DO $$ 
DECLARE
    graph_name TEXT;
BEGIN
    -- Check if AGE extension is available
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Apache AGE extension not available - skipping fulltext indexes';
        RETURN;
    END IF;

    -- Find all graphs in the database
    FOR graph_name IN 
        SELECT name FROM ag_catalog.ag_graph
    LOOP
        RAISE NOTICE 'Adding fulltext indexes to graph: %', graph_name;
        
        -- Fulltext index on node_id property
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_node_id_fulltext ON %I._ag_label_vertex 
                 USING gin(to_tsvector(''english'', ag_catalog.agtype_to_json(properties)->>''node_id''))',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created fulltext index on node_id';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create fulltext index: %', SQLERRM;
        END;
        
        -- Trigram index on node_id property (for fuzzy search)
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_node_id_trgm ON %I._ag_label_vertex 
                 USING gin((ag_catalog.agtype_to_json(properties)->>''node_id'') gin_trgm_ops)',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created trigram index on node_id';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create trigram index: %', SQLERRM;
        END;
        
    END LOOP;
    
    RAISE NOTICE 'Fulltext index creation completed';
    
END $$;

-- Usage examples:
-- 
-- Full-text search with ranking:
--   SELECT ag_catalog.agtype_to_json(properties)->>'node_id' as label,
--          ts_rank(to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id'),
--                  plainto_tsquery('english', 'search term')) as rank
--   FROM <graph>._ag_label_vertex
--   WHERE to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id')
--         @@ plainto_tsquery('english', 'search term')
--   ORDER BY rank DESC;
--
-- Trigram similarity search (fuzzy):
--   SELECT ag_catalog.agtype_to_json(properties)->>'node_id' as label,
--          similarity(ag_catalog.agtype_to_json(properties)->>'node_id', 'search term') as sim
--   FROM <graph>._ag_label_vertex
--   WHERE ag_catalog.agtype_to_json(properties)->>'node_id' % 'search term'
--   ORDER BY sim DESC;
