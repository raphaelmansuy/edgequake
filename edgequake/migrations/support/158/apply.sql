-- ============================================================================
-- Migration 158: Restore orphaned document lineage on AGE graphs
-- Version: 1.0.0 — 2026-09-25
-- SPEC-149 document-scoped lineage (graph page filter).
--
-- PURPOSE:
--   Pre-SPEC-149 retain-on-delete dropped a surviving document's bare
--   document-id token from `source_ids` while keeping the document in
--   `source_document_ids`. Those nodes/edges are invisible to the document's
--   scoped graph (discovery probes `source_ids` / `source_chunk_ids` via GIN).
--   Measured on a 120k-node dev graph: 3 live documents, ~1.8k rows, one
--   document reduced to 10 of 538 entities.
--
-- REPAIR:
--   For every row listing a live document D (row in public.documents, same
--   workspace when the row carries `workspace_id`) in `source_document_ids`
--   with no lineage token for D (neither `D` nor `D-chunk-N`), add the bare
--   token `D` to both `source_ids` and `source_chunk_ids` (the canonical
--   mirror). Chunk ids are not recoverable; the bare token is the form legacy
--   writers used and discovery / count / retain-on-delete all honour.
--
-- SAFETY:
--   * Missing-only: rows with any token for D are untouched; ids of deleted
--     documents are never resurrected.
--   * Idempotent: a repaired row no longer matches, so a second run updates 0.
--   * AGE or public.documents absent → clean skip.
--   * One full scan per label (~20 s at 120k rows); updates touch only the
--     orphaned rows.
--   * Rollback: remove the bare document-id tokens from the two arrays
--     (they are exactly the tokens without a `-chunk-` suffix).
--
-- SSOT body also at migrations/support/158/apply.sql for ops re-runs.
-- ============================================================================

SET search_path = public;
SET statement_timeout = 0;
SET lock_timeout = '30s';

DO $$
DECLARE
  v_graph text;
  v_label text;
  v_updated bigint;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'M158: AGE not installed — skipping orphan lineage repair';
    RETURN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'M158: ag_catalog missing — skipping orphan lineage repair';
    RETURN;
  END IF;
  IF to_regclass('public.documents') IS NULL THEN
    RAISE NOTICE 'M158: public.documents missing — skipping orphan lineage repair';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    FOREACH v_label IN ARRAY ARRAY['Node', 'EDGE']
    LOOP
      IF to_regclass(format('%I.%I', v_graph, v_label)) IS NULL THEN
        CONTINUE;
      END IF;
      EXECUTE format(
        $q$
        WITH lineage_rows AS (
          SELECT t.ctid AS rid,
                 (ag_catalog.agtype_to_json(t.properties))::jsonb AS j
          FROM %I.%I t
          WHERE jsonb_typeof(
                  (ag_catalog.agtype_to_json(t.properties))::jsonb -> 'source_document_ids'
                ) = 'array'
        ),
        tokens AS (
          SELECT r.rid, r.j,
                 (CASE WHEN jsonb_typeof(r.j -> 'source_ids') = 'array'
                       THEN r.j -> 'source_ids' ELSE '[]'::jsonb END)
                 || (CASE WHEN jsonb_typeof(r.j -> 'source_chunk_ids') = 'array'
                          THEN r.j -> 'source_chunk_ids' ELSE '[]'::jsonb END) AS lineage
          FROM lineage_rows r
        ),
        orphans AS (
          SELECT tk.rid, tk.j, tk.lineage, array_agg(DISTINCT d.doc) AS docs
          FROM tokens tk
          CROSS JOIN LATERAL jsonb_array_elements_text(tk.j -> 'source_document_ids') AS d(doc)
          JOIN public.documents pd
            ON pd.id = CASE
                 WHEN d.doc ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                 THEN d.doc::uuid
               END
           AND (tk.j ->> 'workspace_id' IS NULL
                OR tk.j ->> 'workspace_id' = pd.workspace_id::text)
          WHERE NOT EXISTS (
            SELECT 1 FROM jsonb_array_elements_text(tk.lineage) AS x(tok)
            WHERE x.tok = d.doc OR starts_with(x.tok, d.doc || '-chunk-')
          )
          GROUP BY tk.rid, tk.j, tk.lineage
        ),
        rewritten AS (
          SELECT o.rid,
                 jsonb_set(
                   jsonb_set(o.j, '{source_ids}', m.merged, true),
                   '{source_chunk_ids}', m.merged, true
                 ) AS new_j
          FROM orphans o
          CROSS JOIN LATERAL (
            SELECT jsonb_agg(u.tok ORDER BY u.tok COLLATE "C") AS merged
            FROM (
              SELECT jsonb_array_elements_text(o.lineage) AS tok
              UNION
              SELECT unnest(o.docs)
            ) u
          ) m
        )
        UPDATE %I.%I t
        SET properties = r.new_j::text::ag_catalog.agtype
        FROM rewritten r
        WHERE t.ctid = r.rid
        $q$,
        v_graph, v_label, v_graph, v_label
      );
      GET DIAGNOSTICS v_updated = ROW_COUNT;
      IF v_updated > 0 THEN
        RAISE NOTICE 'M158: %.% restored lineage on % rows', v_graph, v_label, v_updated;
      END IF;
    END LOOP;
  END LOOP;

  RAISE NOTICE 'M158: orphan document lineage repair COMPLETE';
END
$$;
