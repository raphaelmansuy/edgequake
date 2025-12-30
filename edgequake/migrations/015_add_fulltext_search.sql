-- Add full-text search index for node_id property
-- Migration: 015_add_fulltext_search.sql
-- Purpose: Enable fuzzy search and autocomplete for entity names
-- Performance: Enables ts_rank scoring and @@ operator matching

-- Full-text search index on node_id using GIN (Generalized Inverted Index)
-- This enables fast text search with ts_rank scoring
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_node_id_fulltext
ON ag_catalog._ag_label_vertex
USING gin(to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id'));

-- Additional index for simple prefix matching (faster than full-text for autocomplete)
-- Uses pg_trgm extension for trigram similarity
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_node_id_trgm
ON ag_catalog._ag_label_vertex
USING gin((ag_catalog.agtype_to_json(properties)->>'node_id') gin_trgm_ops);

-- Usage examples:
-- 
-- Full-text search with ranking:
--   SELECT ag_catalog.agtype_to_json(properties)->>'node_id' as label,
--          ts_rank(to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id'),
--                  plainto_tsquery('english', 'search term')) as rank
--   FROM ag_catalog._ag_label_vertex
--   WHERE to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id')
--         @@ plainto_tsquery('english', 'search term')
--   ORDER BY rank DESC;
--
-- Trigram similarity search (fuzzy):
--   SELECT ag_catalog.agtype_to_json(properties)->>'node_id' as label,
--          similarity(ag_catalog.agtype_to_json(properties)->>'node_id', 'search term') as sim
--   FROM ag_catalog._ag_label_vertex
--   WHERE ag_catalog.agtype_to_json(properties)->>'node_id' % 'search term'
--   ORDER BY sim DESC;
