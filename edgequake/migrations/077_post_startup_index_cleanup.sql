-- ============================================================================
-- Migration 077: Post-startup cleanup — remove indexes recreated by old binary
-- Version: 1.0.0 — 2026-06-30
--
-- PURPOSE:
--   Migrations 068-073 (Sprint 1+2) dropped redundant/unused indexes. However,
--   the application startup code (ensure_indexes, bootstrap_concurrent_indexes,
--   kv.rs DDL, ddl.rs DDL) ran with the OLD code and RECREATED those indexes
--   before the source code fixes took effect.
--
--   This migration drops them again — permanently, now that the application
--   source code has been fixed to not recreate them on the next boot.
--
-- WHAT IS FIXED IN SOURCE CODE (ensures these stay gone after this migration):
--   graph_lifecycle.rs ensure_indexes()         → no longer creates _ag_ parent
--                                                  indexes or old accessor forms
--   graph_lifecycle.rs bootstrap_concurrent_indexes() → creates UNIQUE instead of btree
--   kv.rs                                        → no longer creates kv_value_gin
--   vector/ddl.rs                               → no longer creates metadata_idx
--   migrations/support/045/apply.sql            → no longer creates idx_ dup FTS
--
-- TRANSACTION SAFETY:
--   * Regular DROP INDEX (no CONCURRENTLY) — transaction-safe.
--   * All DROPs use IF EXISTS — idempotent, safe to re-run.
--
-- AFFECTED INDEXES:
--   • eq_*_kv_value_gin              (KV GIN — 112 MB/workspace, 0 scans)
--   • eq_*_vectors_metadata_idx      (vector metadata GIN — 13 MB, 0 scans)
--   • idx_eq_*_vectors_content_tsv   (duplicate FTS — 1.9 MB each)
--   • idx_node_prop_node_id          (old agtype_access_operator form, 0 scans)
--   • idx_node_prop_node_id_btree    (non-unique btree — superseded by UNIQUE)
--   • idx_edge_start_end             (composite EDGE btree, 0 scans)
--   • idx_edge_props_gin             (EDGE GIN, 0 scans)
--   • idx_ag_*                       (parent-table indexes, 0 rows in parent)
-- ============================================================================

-- ── 1. KV GIN value index ─────────────────────────────────────────────────
DO $$
DECLARE v_tbl text; v_idx text;
BEGIN
  FOR v_tbl IN SELECT tablename FROM pg_tables
               WHERE schemaname='public' AND tablename LIKE 'eq_%_kv'
  LOOP
    v_idx := v_tbl || '_value_gin';
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public' AND indexname=v_idx) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_idx);
      RAISE NOTICE 'M077: Dropped KV GIN %', v_idx;
    END IF;
  END LOOP;
END $$;

-- ── 2. Vector metadata GIN index ──────────────────────────────────────────
DO $$
DECLARE v_tbl text; v_idx text;
BEGIN
  FOR v_tbl IN SELECT tablename FROM pg_tables
               WHERE schemaname='public' AND tablename LIKE 'eq_%_vectors'
                 AND tablename NOT LIKE '%_stats'
  LOOP
    v_idx := v_tbl || '_metadata_idx';
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public' AND indexname=v_idx) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_idx);
      RAISE NOTICE 'M077: Dropped vector metadata GIN %', v_idx;
    END IF;
  END LOOP;
END $$;

-- ── 3. Duplicate FTS index (idx_ prefix form) ──────────────────────────────
DO $$
DECLARE v_tbl text; v_idx text;
BEGIN
  FOR v_tbl IN SELECT tablename FROM pg_tables
               WHERE schemaname='public' AND tablename LIKE 'eq_%_vectors'
                 AND tablename NOT LIKE '%_stats'
  LOOP
    v_idx := 'idx_' || v_tbl || '_content_tsv';
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public' AND indexname=v_idx) THEN
      -- Safety: only drop if canonical form still exists
      IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public'
                 AND indexname = v_tbl || '_content_tsv_idx') THEN
        EXECUTE format('DROP INDEX IF EXISTS public.%I', v_idx);
        RAISE NOTICE 'M077: Dropped duplicate FTS %', v_idx;
      ELSE
        RAISE NOTICE 'M077: Skipping % — canonical FTS absent', v_idx;
      END IF;
    END IF;
  END LOOP;
END $$;

-- ── 4. Graph indexes — Node and EDGE child tables + parent tables ──────────
DO $$
DECLARE v_graph text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname='age') THEN
    RAISE NOTICE 'M077: AGE not installed — skipping graph index cleanup'; RETURN;
  END IF;

  FOR v_graph IN SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    -- Remove old accessor form superseded by UNIQUE index (Migration 074)
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_node_prop_node_id') THEN
      EXECUTE format('DROP INDEX IF EXISTS %I.idx_node_prop_node_id', v_graph);
      RAISE NOTICE 'M077: Dropped idx_node_prop_node_id in %', v_graph;
    END IF;

    -- Remove non-unique btree superseded by UNIQUE index (Migration 074)
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_node_prop_node_id_btree') AND
       EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_node_prop_node_id_unique')
    THEN
      EXECUTE format('DROP INDEX IF EXISTS %I.idx_node_prop_node_id_btree', v_graph);
      RAISE NOTICE 'M077: Dropped idx_node_prop_node_id_btree in %', v_graph;
    END IF;

    -- Remove composite EDGE index (0 scans)
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_edge_start_end') THEN
      EXECUTE format('DROP INDEX IF EXISTS %I.idx_edge_start_end', v_graph);
      RAISE NOTICE 'M077: Dropped idx_edge_start_end in %', v_graph;
    END IF;

    -- Remove EDGE GIN (0 scans)
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_edge_props_gin') THEN
      EXECUTE format('DROP INDEX IF EXISTS %I.idx_edge_props_gin', v_graph);
      RAISE NOTICE 'M077: Dropped idx_edge_props_gin in %', v_graph;
    END IF;

    -- Remove all _ag_label_vertex parent-table indexes (0 rows in parent)
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_vertex_props_gin';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_vertex_props_gin', v_graph); END IF;
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_vertex_tenant_id';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_vertex_tenant_id', v_graph); END IF;
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_vertex_workspace_id';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_vertex_workspace_id', v_graph); END IF;

    -- Remove all _ag_label_edge parent-table indexes (0 rows in parent)
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_edge_start_id';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_edge_start_id', v_graph); END IF;
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_edge_end_id';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_edge_end_id', v_graph); END IF;
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_edge_start_end';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_edge_start_end', v_graph); END IF;
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_edge_source_id';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_edge_source_id', v_graph); END IF;
    PERFORM 1 FROM pg_indexes WHERE schemaname=v_graph AND indexname='idx_ag_edge_target_id';
    IF FOUND THEN EXECUTE format('DROP INDEX IF EXISTS %I.idx_ag_edge_target_id', v_graph); END IF;

    RAISE NOTICE 'M077: Graph % cleanup complete', v_graph;
  END LOOP;
END $$;
