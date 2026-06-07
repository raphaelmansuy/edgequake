-- SPEC-006 Migration 038 — Rollback (drops indexes only; NO data loss)
-- Safe to run multiple times (IF EXISTS).
--
-- Usage:
--   edgequake/scripts/migrations/apply_038.sh --rollback --yes

DO $$
DECLARE
    graph_name text;
    graph_schema text;
    idx_prefix text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'AGE not installed — nothing to rollback';
        RETURN;
    END IF;

    FOR graph_name IN SELECT name FROM ag_catalog.ag_graph LOOP
        graph_schema := graph_name;
        idx_prefix := replace(graph_name, '.', '_');

        IF to_regclass(format('%I._ag_label_vertex', graph_schema)) IS NULL THEN
            RAISE NOTICE 'Skip graph % — vertex table missing', graph_name;
            CONTINUE;
        END IF;

        EXECUTE format('DROP INDEX IF EXISTS %I.idx_%s_vertex_source_id', graph_schema, idx_prefix);
        EXECUTE format('DROP INDEX IF EXISTS %I.idx_%s_vertex_source_ids_gin', graph_schema, idx_prefix);

        IF to_regclass(format('%I._ag_label_edge', graph_schema)) IS NOT NULL THEN
            EXECUTE format('DROP INDEX IF EXISTS %I.idx_%s_edge_source_ids_gin', graph_schema, idx_prefix);
        END IF;

        RAISE NOTICE 'Rolled back 038 indexes for graph %', graph_name;
    END LOOP;
END $$;
