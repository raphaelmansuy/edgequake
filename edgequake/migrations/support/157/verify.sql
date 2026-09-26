-- M157 verify: every existing lineage GIN index carries gin_pending_list_limit=256.
DO $$
DECLARE
  v_bad text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'M157 verify: AGE not installed';
    RETURN;
  END IF;
  SELECT string_agg(format('%s.%s', n.nspname, c.relname), ', ' ORDER BY n.nspname, c.relname)
    INTO v_bad
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    JOIN ag_catalog.ag_graph g ON g.name = n.nspname
   WHERE c.relkind = 'i'
     AND c.relname IN (
       'idx_node_source_ids_gin', 'idx_node_source_chunk_ids_gin',
       'idx_edge_source_ids_gin', 'idx_edge_source_chunk_ids_gin'
     )
     AND NOT (COALESCE(c.reloptions, '{}') @> ARRAY['gin_pending_list_limit=256']);
  IF v_bad IS NOT NULL THEN
    RAISE EXCEPTION 'M157 verify FAILED: missing gin_pending_list_limit=256 on %', v_bad;
  END IF;
  RAISE NOTICE 'M157 verify OK';
END
$$;
