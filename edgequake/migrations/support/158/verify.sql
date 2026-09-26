-- M158 verify: no row lists a live document in source_document_ids without a
-- lineage token for it (bare id or `{doc}-chunk-N`) in source_ids/source_chunk_ids.
DO $$
DECLARE
  v_graph text;
  v_label text;
  v_bad bigint;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'M158 verify: AGE not installed';
    RETURN;
  END IF;
  FOR v_graph IN SELECT name FROM ag_catalog.ag_graph ORDER BY name LOOP
    FOREACH v_label IN ARRAY ARRAY['Node', 'EDGE'] LOOP
      IF to_regclass(format('%I.%I', v_graph, v_label)) IS NULL THEN
        CONTINUE;
      END IF;
      EXECUTE format(
        $q$
        SELECT count(*)
        FROM (SELECT (ag_catalog.agtype_to_json(t.properties))::jsonb AS j FROM %I.%I t) r
        CROSS JOIN LATERAL jsonb_array_elements_text(
          CASE WHEN jsonb_typeof(r.j -> 'source_document_ids') = 'array'
               THEN r.j -> 'source_document_ids' ELSE '[]'::jsonb END
        ) AS d(doc)
        JOIN public.documents pd
          ON pd.id = CASE
               WHEN d.doc ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
               THEN d.doc::uuid
             END
         AND (r.j ->> 'workspace_id' IS NULL OR r.j ->> 'workspace_id' = pd.workspace_id::text)
        WHERE NOT EXISTS (
          SELECT 1 FROM jsonb_array_elements_text(
            (CASE WHEN jsonb_typeof(r.j -> 'source_ids') = 'array'
                  THEN r.j -> 'source_ids' ELSE '[]'::jsonb END)
            || (CASE WHEN jsonb_typeof(r.j -> 'source_chunk_ids') = 'array'
                     THEN r.j -> 'source_chunk_ids' ELSE '[]'::jsonb END)
          ) AS x(tok)
          WHERE x.tok = d.doc OR starts_with(x.tok, d.doc || '-chunk-')
        )
        $q$,
        v_graph, v_label
      ) INTO v_bad;
      IF v_bad > 0 THEN
        RAISE EXCEPTION 'M158 verify FAILED: %.% has % orphaned live-document lineage pairs',
          v_graph, v_label, v_bad;
      END IF;
    END LOOP;
    RAISE NOTICE 'M158 verify OK: %', v_graph;
  END LOOP;
END
$$;
