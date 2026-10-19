-- Backfill `relationships.source_chunk_keys` (migration 211) from the exact
-- per-edge lineage in `eq_graph_edges.properties -> 'source_chunk_ids'`.
--
-- Staged through temp tables that are ANALYZEd before they are joined. A
-- single statement over `jsonb_array_elements_text` is estimated at 4 rows
-- (177,001 actual), so the planner nests loops around it: 450 s on 155,380
-- relationships. Staged, the same backfill is 14 s and fills the same rows.
--
-- Edges are keyed `{workspace}::{NAME}`; the prefix is stripped so the final
-- join is a plain four-column equality (workspace, source, target, type).
-- Idempotent: only rows whose keys are still NULL are written. Relationships
-- with no matching edge keep NULL and stay out of document-scoped search,
-- the engine's rule for rows without lineage.

DO $$
DECLARE
    filled bigint;
BEGIN
    IF to_regclass('public.eq_graph_edges') IS NULL THEN
        RAISE NOTICE '212: eq_graph_edges absent, nothing to backfill';
        RETURN;
    END IF;

    DROP TABLE IF EXISTS bf_edge_keys, bf_edge_agg;

    CREATE TEMP TABLE bf_edge_keys ON COMMIT DROP AS
    SELECT e.workspace_id,
           substr(e.source_id, length(e.workspace_id) + 3) AS src,
           substr(e.target_id, length(e.workspace_id) + 3) AS tgt,
           e.rel_type,
           k.key
      FROM public.eq_graph_edges e
      CROSS JOIN LATERAL jsonb_array_elements_text(
            CASE WHEN jsonb_typeof(e.properties -> 'source_chunk_ids') = 'array'
                 THEN e.properties -> 'source_chunk_ids' ELSE '[]'::jsonb END) AS k(key)
     WHERE e.source_id LIKE e.workspace_id || '::%'
       AND e.target_id LIKE e.workspace_id || '::%';
    ANALYZE bf_edge_keys;

    CREATE TEMP TABLE bf_edge_agg ON COMMIT DROP AS
    SELECT workspace_id, src, tgt, rel_type, array_agg(DISTINCT key) AS keys
      FROM bf_edge_keys
     GROUP BY 1, 2, 3, 4;
    ANALYZE bf_edge_agg;

    UPDATE public.relationships r
       SET source_chunk_keys = a.keys
      FROM public.entities es, public.entities et, bf_edge_agg a
     WHERE r.source_chunk_keys IS NULL
       AND es.id = r.source_id
       AND et.id = r.target_id
       AND a.workspace_id = r.workspace_id::text
       AND a.src = es.name
       AND a.tgt = et.name
       AND a.rel_type = r.relation_type;
    GET DIAGNOSTICS filled = ROW_COUNT;
    RAISE NOTICE '212: backfilled source_chunk_keys on % relationships', filled;
END
$$;
