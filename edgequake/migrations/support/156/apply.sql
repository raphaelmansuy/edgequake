-- ============================================================================
-- Migration 156: Backfill missing source_ids / source_document_ids on AGE graphs
-- Version: 1.0.0 — 2026-09-25
-- SPEC-149 document-scoped lineage (graph page filter).
--
-- PURPOSE:
--   Durable-committer / projection writes historically emitted only
--   `source_chunk_ids`. Document lineage, cascade delete, and entity-count
--   probes match on `source_ids` via GIN. Backfill the missing mirror and
--   derive `source_document_ids` when absent.
--
-- SAFETY:
--   * Missing-only: never rewrites rows that already have a `source_ids` array.
--   * Idempotent: second run updates 0 rows.
--   * AGE absent → clean skip.
--   * Batched by ctid (10k) to avoid long exclusive locks.
--   * Rollback is a no-op (additive keys; readers accept both shapes).
--
-- SSOT body also at migrations/support/156/apply.sql for ops re-runs.
-- ============================================================================

SET search_path = public;
SET statement_timeout = 0;
SET lock_timeout = '30s';

DO $$
DECLARE
  v_graph text;
  v_batch int := 10000;
  v_updated bigint;
  v_total_nodes bigint;
  v_total_edges bigint;
  v_label text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'M156: AGE not installed — skipping lineage backfill';
    RETURN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'M156: ag_catalog missing — skipping lineage backfill';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    IF NOT EXISTS (
      SELECT 1 FROM pg_tables WHERE schemaname = v_graph AND tablename = 'Node'
    ) OR NOT EXISTS (
      SELECT 1 FROM pg_tables WHERE schemaname = v_graph AND tablename = 'EDGE'
    ) THEN
      RAISE NOTICE 'M156: Node/EDGE pending on % — skip', v_graph;
      CONTINUE;
    END IF;

    v_total_nodes := 0;
    v_total_edges := 0;

    FOREACH v_label IN ARRAY ARRAY['Node', 'EDGE']
    LOOP
      LOOP
        EXECUTE format(
          $q$
          WITH batch AS (
            SELECT t.ctid AS rid,
                   (ag_catalog.agtype_to_json(t.properties))::jsonb AS j
            FROM %I.%I t
            WHERE jsonb_typeof(
                    (ag_catalog.agtype_to_json(t.properties))::jsonb -> 'source_chunk_ids'
                  ) = 'array'
              AND jsonb_typeof(
                    (ag_catalog.agtype_to_json(t.properties))::jsonb -> 'source_ids'
                  ) IS DISTINCT FROM 'array'
            LIMIT %s
          ),
          rewritten AS (
            SELECT
              b.rid,
              jsonb_set(
                CASE
                  WHEN jsonb_typeof(b.j -> 'source_document_ids') = 'array' THEN b.j
                  ELSE jsonb_set(
                    b.j,
                    '{source_document_ids}',
                    COALESCE(
                      (
                        SELECT jsonb_agg(to_jsonb(d) ORDER BY d)
                        FROM (
                          SELECT DISTINCT NULLIF(doc_id, '') AS d
                          FROM (
                            SELECT b.j ->> 'source_document_id' AS doc_id
                            UNION ALL
                            SELECT regexp_replace(c, '-chunk-[0-9]+$', '')
                            FROM jsonb_array_elements_text(
                              COALESCE(b.j -> 'source_chunk_ids', '[]'::jsonb)
                            ) AS u(c)
                            WHERE c ~ '-chunk-[0-9]+$'
                          ) raw
                        ) docs
                        WHERE d IS NOT NULL
                      ),
                      '[]'::jsonb
                    ),
                    true
                  )
                END,
                '{source_ids}',
                COALESCE(b.j -> 'source_chunk_ids', '[]'::jsonb),
                true
              ) AS new_j
            FROM batch b
          )
          UPDATE %I.%I t
          SET properties = r.new_j::text::ag_catalog.agtype
          FROM rewritten r
          WHERE t.ctid = r.rid
          $q$,
          v_graph, v_label, v_batch, v_graph, v_label
        );
        GET DIAGNOSTICS v_updated = ROW_COUNT;
        IF v_label = 'Node' THEN
          v_total_nodes := v_total_nodes + v_updated;
        ELSE
          v_total_edges := v_total_edges + v_updated;
        END IF;
        EXIT WHEN v_updated = 0;
        RAISE NOTICE 'M156: %.% batch % rows', v_graph, v_label, v_updated;
      END LOOP;
    END LOOP;

    RAISE NOTICE 'M156: % complete — nodes=% edges=%', v_graph, v_total_nodes, v_total_edges;
  END LOOP;

  RAISE NOTICE 'M156: lineage source_ids backfill COMPLETE';
END
$$;
