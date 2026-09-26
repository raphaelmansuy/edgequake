-- M156 verify: zero rows with source_chunk_ids array but missing source_ids array.
DO $$
DECLARE
  v_graph text;
  v_bad bigint;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'M156 verify: AGE not installed';
    RETURN;
  END IF;
  FOR v_graph IN SELECT name FROM ag_catalog.ag_graph ORDER BY name LOOP
    IF to_regclass(format('%I."Node"', v_graph)) IS NULL THEN
      CONTINUE;
    END IF;
    EXECUTE format(
      $q$SELECT count(*) FROM %I."Node"
         WHERE jsonb_typeof((ag_catalog.agtype_to_json(properties))::jsonb -> 'source_chunk_ids') = 'array'
           AND jsonb_typeof((ag_catalog.agtype_to_json(properties))::jsonb -> 'source_ids') IS DISTINCT FROM 'array'$q$,
      v_graph
    ) INTO v_bad;
    IF v_bad > 0 THEN
      RAISE EXCEPTION 'M156 verify FAILED: %.Node still missing source_ids on % rows', v_graph, v_bad;
    END IF;
    EXECUTE format(
      $q$SELECT count(*) FROM %I."EDGE"
         WHERE jsonb_typeof((ag_catalog.agtype_to_json(properties))::jsonb -> 'source_chunk_ids') = 'array'
           AND jsonb_typeof((ag_catalog.agtype_to_json(properties))::jsonb -> 'source_ids') IS DISTINCT FROM 'array'$q$,
      v_graph
    ) INTO v_bad;
    IF v_bad > 0 THEN
      RAISE EXCEPTION 'M156 verify FAILED: %.EDGE still missing source_ids on % rows', v_graph, v_bad;
    END IF;
    RAISE NOTICE 'M156 verify OK: %', v_graph;
  END LOOP;
END
$$;
